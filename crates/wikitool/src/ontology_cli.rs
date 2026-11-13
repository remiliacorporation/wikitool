use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde_json::Value;
use wikitool_core::ontology::{self, ExportOptions};
use wikitool_core::site::adapter::load_site_adapter;

use crate::RuntimeOptions;
use crate::cli_support::{normalize_path, resolve_runtime_paths};

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
}

#[derive(Debug, Args)]
struct GenerateArgs {
    #[arg(long, help = "Fail if the module is out of date, without writing")]
    check: bool,
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
            println!(
                "exported {} at {} to {} and {}",
                normalize_path(&args.from),
                export["source"]["commit"].as_str().unwrap_or("?"),
                ontology::EXPORT_PATH,
                module_relative
            );
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
            if args.check {
                if !up_to_date {
                    bail!("{module_relative} is out of date; run `wikitool ontology generate`");
                }
                println!("{module_relative} is up to date");
            } else if up_to_date {
                println!("{module_relative} is up to date");
            } else {
                write(&module_path, &module)?;
                println!("wrote {module_relative}");
            }
        }
    }
    Ok(())
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", normalize_path(parent)))?;
    }
    fs::write(path, text).with_context(|| format!("failed to write {}", normalize_path(path)))
}
