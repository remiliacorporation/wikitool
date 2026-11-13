use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::Serialize;
use serde_json::Value;
use wikitool_core::ontology::{self, ExportOptions, typing};
use wikitool_core::site::adapter::load_site_adapter;

use crate::RuntimeOptions;
use crate::cli_support::{OutputFormat, normalize_path, resolve_runtime_paths};

#[derive(Debug, Args)]
pub(crate) struct OntologyArgs {
    #[command(subcommand)]
    command: OntologySubcommand,
}

#[derive(Debug, Subcommand)]
enum OntologySubcommand {
    #[command(
        about = "Export an ontology checkout into the vendored copy and regenerate the module"
    )]
    Export(ExportArgs),
    #[command(about = "Generate Module:Ontology/data from the vendored copy")]
    Generate(GenerateArgs),
    #[command(
        about = "Check Module:Ontology/local and Module:Ontology/display against the vendored copy"
    )]
    Check(CheckArgs),
    #[command(about = "Report how the wiki's articles use the ontology, as evidence for an intake")]
    Report(CheckArgs),
}

#[derive(Debug, Args)]
struct ExportArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "The ontology directory (with registry/ and mappings.sssom.tsv) in a clean git checkout"
    )]
    from: PathBuf,
    #[arg(
        long,
        value_name = "NAME",
        help = "The wiki's product name in the ontology's product-uses.tsv; defaults to the site adapter's"
    )]
    product: Option<String>,
    #[arg(
        long,
        value_name = "PATH",
        help = "The directory with product-uses.tsv and the pinned extracts, relative to --from; defaults to the site adapter's [ontology] imports"
    )]
    imports: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        value_name = "FORMAT",
        help = "Output format: text|json"
    )]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct GenerateArgs {
    #[arg(long, help = "Fail if the module is out of date, without writing")]
    check: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        value_name = "FORMAT",
        help = "Output format: text|json"
    )]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct CheckArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        value_name = "FORMAT",
        help = "Output format: text|json"
    )]
    format: OutputFormat,
}

#[derive(Debug, Serialize)]
struct CheckReport {
    schema_version: &'static str,
    /// `not_ready` without `[ontology]` or a vendored copy, `incomplete` when
    /// a module is missing or unreadable, otherwise `clean` or `findings`.
    status: &'static str,
    export_ready: bool,
    commit: Option<String>,
    export_path: String,
    modules: Vec<typing::ModuleStatus>,
    checks_run: Vec<&'static str>,
    skipped: Vec<typing::SkippedCheck>,
    finding_count: usize,
    findings: Vec<typing::TypingFinding>,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportReport {
    schema_version: &'static str,
    ontology_dir: String,
    commit: String,
    product: String,
    export_path: String,
    module_path: String,
}

#[derive(Debug, Serialize)]
struct GenerateReport {
    schema_version: &'static str,
    /// `not_ready` without a vendored copy, otherwise `up_to_date`,
    /// `out_of_date` (with `--check`) or `written`.
    status: &'static str,
    /// Whether the project has a vendored copy to generate from.
    export_ready: bool,
    commit: Option<String>,
    product: Option<String>,
    export_path: String,
    module_path: String,
    /// The vendored copy's top-level sections the module holds: all of them,
    /// or those the site adapter's `[ontology] module_sections` selects plus
    /// `source`. Null without a vendored copy.
    sections: Option<Vec<String>>,
    check: bool,
    /// Whether the module on disk already matched the vendored copy.
    up_to_date: bool,
    written: bool,
    message: Option<&'static str>,
}

/// Why `generate` cannot run in a project that never vendored an ontology.
const NO_VENDORED_COPY: &str = "this project has no vendored ontology at \
    vendor/ontology/ontology.json; `wikitool ontology export --from <dir>` produces it from a \
    checkout of the ontology's source repository. A wiki that does not use an ontology needs \
    neither command.";

pub(crate) fn run_ontology(runtime: &RuntimeOptions, args: OntologyArgs) -> Result<()> {
    let paths = resolve_runtime_paths(runtime)?;
    let module_relative = ontology::module_relative_path(&paths)?;
    let export_path = paths.project_root.join(ontology::EXPORT_PATH);
    let module_path = paths.project_root.join(&module_relative);
    match args.command {
        OntologySubcommand::Export(args) => {
            let rules = load_site_adapter(&paths)?.ontology;
            let product = args
                .product
                .or_else(|| rules.as_ref().map(|rules| rules.product.clone()))
                .context("pass --product, or name the ontology in the site adapter's [ontology]")?;
            let imports = args
                .imports
                .or_else(|| rules.as_ref()?.imports.as_ref().map(PathBuf::from))
                .context(
                    "pass --imports, or set imports in the site adapter's [ontology], \
                     to name the directory with product-uses.tsv and the pinned extracts",
                )?;
            let options = ExportOptions {
                product: &product,
                imports: &imports,
                excluded_rename_prefixes: rules
                    .as_ref()
                    .map_or(&[][..], |rules| &rules.exclude_rename_prefixes),
                source_identifier: rules.as_ref().map(|rules| rules.source.as_str()),
                local: rules
                    .as_ref()
                    .map(|rules| (rules.local_prefix.as_str(), rules.local_namespace.as_str())),
            };
            let export = ontology::export(&args.from, &options)?;
            // Render both outputs before writing either. The vendored copy is
            // always complete; only the module follows `module_sections`.
            let export_text = serde_json::to_string_pretty(&export)? + "\n";
            let module = ontology::generate_module(
                &export,
                rules
                    .as_ref()
                    .and_then(|rules| rules.module_sections.as_deref()),
            )?;
            write(&export_path, &export_text)?;
            write(&module_path, &module.text)?;
            let report = ExportReport {
                schema_version: "ontology_export_v1",
                ontology_dir: normalize_path(&args.from),
                commit: commit_of(&export),
                product,
                export_path: ontology::EXPORT_PATH.to_string(),
                module_path: module_relative,
            };
            if args.format.is_json() {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "exported {} at {} to {} and {}",
                    report.ontology_dir, report.commit, report.export_path, report.module_path
                );
            }
        }
        OntologySubcommand::Generate(args) => {
            let module_sections = load_site_adapter(&paths)?
                .ontology
                .and_then(|rules| rules.module_sections);
            let text = match fs::read_to_string(&export_path) {
                Ok(text) => text,
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    if args.format.is_json() {
                        let report = GenerateReport {
                            schema_version: "ontology_generate_v1",
                            status: "not_ready",
                            export_ready: false,
                            commit: None,
                            product: None,
                            export_path: ontology::EXPORT_PATH.to_string(),
                            module_path: module_relative,
                            sections: None,
                            check: args.check,
                            up_to_date: false,
                            written: false,
                            message: Some(NO_VENDORED_COPY),
                        };
                        println!("{}", serde_json::to_string_pretty(&report)?);
                        bail!("ontology generate requires a vendored ontology");
                    }
                    bail!("{NO_VENDORED_COPY}");
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to read {}", normalize_path(&export_path))
                    });
                }
            };
            let export: Value = serde_json::from_str(&text)
                .with_context(|| format!("failed to parse {}", normalize_path(&export_path)))?;
            let ontology::GeneratedModule {
                text: module,
                sections,
            } = ontology::generate_module(&export, module_sections.as_deref())?;
            let current = match fs::read_to_string(&module_path) {
                Ok(current) => Some(current),
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to read {}", normalize_path(&module_path))
                    });
                }
            };
            // A checkout with CRLF line endings holds the same module.
            let up_to_date = current.is_some_and(|current| current.replace("\r\n", "\n") == module);
            let written = !args.check && !up_to_date;
            if written {
                write(&module_path, &module)?;
            }
            let report = GenerateReport {
                schema_version: "ontology_generate_v1",
                status: if written {
                    "written"
                } else if up_to_date {
                    "up_to_date"
                } else {
                    "out_of_date"
                },
                export_ready: true,
                commit: Some(commit_of(&export)),
                product: export
                    .pointer("/source/product")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                export_path: ontology::EXPORT_PATH.to_string(),
                module_path: module_relative,
                sections: Some(sections),
                check: args.check,
                up_to_date,
                written,
                message: None,
            };
            if args.format.is_json() {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if written {
                println!("wrote {}", report.module_path);
            } else if up_to_date {
                println!("{} is up to date", report.module_path);
            }
            if args.check && !up_to_date {
                bail!(
                    "{} is out of date; run `wikitool ontology generate`",
                    report.module_path
                );
            }
        }
        OntologySubcommand::Check(args) => run_check(&paths, &args)?,
        OntologySubcommand::Report(args) => run_report(&paths, &args)?,
    }
    Ok(())
}

/// Check the wiki's ontology modules alone, for CI: every local term is
/// mapped and shadows no shared term, and every display field and type
/// resolves. Fails on any finding or skipped check.
fn run_check(paths: &wikitool_core::runtime::ResolvedPaths, args: &CheckArgs) -> Result<()> {
    let adapter = load_site_adapter(paths)?;
    let mut report = CheckReport {
        schema_version: "ontology_check_v1",
        status: "not_ready",
        export_ready: false,
        commit: None,
        export_path: ontology::EXPORT_PATH.to_string(),
        modules: Vec::new(),
        checks_run: Vec::new(),
        skipped: Vec::new(),
        finding_count: 0,
        findings: Vec::new(),
        message: None,
    };
    let project = match typing::load_project_ontology(paths, &adapter)? {
        typing::OntologyAvailability::NotConfigured => {
            report.message = Some(
                "the site adapter has no [ontology] section; a wiki that does not use an \
                 ontology needs no ontology check"
                    .to_string(),
            );
            None
        }
        typing::OntologyAvailability::Unavailable { reason } => {
            report.message = Some(reason);
            None
        }
        typing::OntologyAvailability::Ready(project) => Some(project),
    };
    if let Some(project) = &project {
        let check = project.check_modules();
        report.export_ready = true;
        report.commit = project.commit().map(str::to_string);
        report.modules = vec![project.local_module.clone(), project.display_module.clone()];
        report.finding_count = check.findings.len();
        report.status = if !check.skipped.is_empty() {
            "incomplete"
        } else if check.findings.is_empty() {
            "clean"
        } else {
            "findings"
        };
        report.checks_run = check.checks_run;
        report.skipped = check.skipped;
        report.findings = check.findings;
    }
    if args.format.is_json() {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        if let Some(message) = &report.message {
            println!("{message}");
        }
        for module in &report.modules {
            println!(
                "{}: {} ({})",
                module.title, module.status, module.relative_path
            );
        }
        for skipped in &report.skipped {
            println!("skipped {}: {}", skipped.check, skipped.reason);
        }
        for finding in &report.findings {
            println!("{}: {}", finding.code, finding.message);
        }
        if report.status == "clean" {
            println!("ontology modules are consistent with the vendored copy");
        }
    }
    match report.status {
        "clean" => Ok(()),
        "not_ready" => bail!("ontology check requires [ontology] and a vendored ontology"),
        "incomplete" => bail!("ontology check could not read every ontology module"),
        _ => bail!("ontology check found {} finding(s)", report.finding_count),
    }
}

fn commit_of(export: &Value) -> String {
    export
        .pointer("/source/commit")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", normalize_path(parent)))?;
    }
    fs::write(path, text).with_context(|| format!("failed to write {}", normalize_path(path)))
}

/// Walk the content mirror and report, for the intake that moves the wiki's
/// terms into the shared ontology: the pages that use each local term, the
/// display fields with no property, the values that fail their value type,
/// and the class ids no module defines.
fn run_report(paths: &wikitool_core::runtime::ResolvedPaths, args: &CheckArgs) -> Result<()> {
    let adapter = load_site_adapter(paths)?;
    let project = match typing::load_project_ontology(paths, &adapter)? {
        typing::OntologyAvailability::NotConfigured => bail!(
            "the site adapter has no [ontology] section; a wiki that does not use an ontology \
             has nothing to report"
        ),
        typing::OntologyAvailability::Unavailable { reason } => bail!("{reason}"),
        typing::OntologyAvailability::Ready(project) => project,
    };
    let pages = wikitool_core::ontology::report::read_content_pages(paths)?;
    let report = project.report(&pages);
    if args.format.is_json() {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!(
        "{} articles read, {} typed",
        report.page_count, report.typed_page_count
    );
    for skipped in &report.skipped {
        println!("skipped {}: {}", skipped.check, skipped.reason);
    }
    println!("local terms:");
    for term in &report.local_terms {
        println!(
            "  {} ({}): {} page(s){}",
            term.id,
            term.section,
            term.page_count,
            if term.mapped { "" } else { ", unmapped" }
        );
    }
    println!("fields without a property:");
    for field in &report.fields_without_property {
        println!("  {}: {} page(s)", field.field, field.page_count);
    }
    println!("values that fail their value type:");
    for failure in &report.value_failures {
        println!(
            "  {} ({}): {} value(s) on {} page(s)",
            failure.property,
            failure.value_type.as_deref().unwrap_or("unknown"),
            failure.failure_count,
            failure.page_count
        );
    }
    println!("unknown classes:");
    for class in &report.unknown_classes {
        println!("  {}: {} page(s)", class.id, class.page_count);
    }
    Ok(())
}
