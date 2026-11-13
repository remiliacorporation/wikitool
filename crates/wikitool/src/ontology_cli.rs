use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use wikitool_core::ontology;
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
}

#[derive(Debug, Args)]
struct GenerateArgs {
    #[arg(long, help = "Fail if the module is out of date, without writing")]
    check: bool,
}

pub(crate) fn run_ontology(runtime: &RuntimeOptions, args: OntologyArgs) -> Result<()> {
    let paths = resolve_runtime_paths(runtime)?;
    let export_path = paths.project_root.join(ontology::EXPORT_PATH);
    let module_path = paths.project_root.join(ontology::MODULE_PATH);
    match args.command {
        OntologySubcommand::Export(args) => {
            let rules = load_site_adapter(&paths)?.ontology;
            let product = args
                .product
                .or_else(|| rules.as_ref().map(|rules| rules.product.clone()))
                .context("pass --product, or name the ontology in the site adapter's [ontology]")?;
            let mut export = ontology::export(&args.from, &product)?;
            if let Some(rules) = rules {
                export["source"]["ontology"] = rules.source.into();
                export["source"]["local"] =
                    serde_json::json!({ rules.local_prefix: rules.local_namespace });
            }
            write(
                &export_path,
                &(serde_json::to_string_pretty(&export)? + "\n"),
            )?;
            write(&module_path, &ontology::generate_module(&export)?)?;
            println!(
                "exported {} at {} to {} and {}",
                normalize_path(&args.from),
                export["source"]["commit"].as_str().unwrap_or("?"),
                ontology::EXPORT_PATH,
                ontology::MODULE_PATH
            );
        }
        OntologySubcommand::Generate(args) => {
            let text = fs::read_to_string(&export_path)
                .with_context(|| format!("failed to read {}", normalize_path(&export_path)))?;
            let export = serde_json::from_str(&text)
                .with_context(|| format!("failed to parse {}", normalize_path(&export_path)))?;
            let module = ontology::generate_module(&export)?;
            if args.check {
                let current = fs::read_to_string(&module_path).unwrap_or_default();
                if current != module {
                    bail!(
                        "{} is out of date; run `wikitool ontology generate`",
                        ontology::MODULE_PATH
                    );
                }
                println!("{} is up to date", ontology::MODULE_PATH);
            } else {
                write(&module_path, &module)?;
                println!("wrote {}", ontology::MODULE_PATH);
            }
        }
    }
    Ok(())
}

fn write(path: &std::path::Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", normalize_path(parent)))?;
    }
    fs::write(path, text).with_context(|| format!("failed to write {}", normalize_path(path)))
}
