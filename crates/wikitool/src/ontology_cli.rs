use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::Serialize;
use serde_json::Value;
use wikitool_core::ontology::{self, ExportOptions};
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
    commit: String,
    product: Option<String>,
    export_path: String,
    module_path: String,
    check: bool,
    /// Whether the module on disk already matched the vendored copy.
    up_to_date: bool,
    written: bool,
}

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
                published_at: rules.as_ref().map(|rules| rules.source.as_str()),
                local: rules
                    .as_ref()
                    .map(|rules| (rules.local_prefix.as_str(), rules.local_namespace.as_str())),
            };
            let export = ontology::export(&args.from, &options)?;
            // Render both outputs before writing either.
            let export_text = serde_json::to_string_pretty(&export)? + "\n";
            let module = ontology::generate_module(&export)?;
            write(&export_path, &export_text)?;
            write(&module_path, &module)?;
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
            let text = fs::read_to_string(&export_path)
                .with_context(|| format!("failed to read {}", normalize_path(&export_path)))?;
            let export: Value = serde_json::from_str(&text)
                .with_context(|| format!("failed to parse {}", normalize_path(&export_path)))?;
            let module = ontology::generate_module(&export)?;
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
                commit: commit_of(&export),
                product: export
                    .pointer("/source/product")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                export_path: ontology::EXPORT_PATH.to_string(),
                module_path: module_relative,
                check: args.check,
                up_to_date,
                written,
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
    }
    Ok(())
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
