use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use wikitool_core::ontology;

use crate::RuntimeOptions;
use crate::cli_support::{normalize_path, resolve_runtime_paths};

#[derive(Debug, Args)]
pub(crate) struct OntologyArgs {
    #[command(subcommand)]
    command: OntologySubcommand,
}

#[derive(Debug, Subcommand)]
enum OntologySubcommand {
    #[command(about = "Export a clean remilia-sdk checkout of the ontology into the vendored copy")]
    Export(ExportArgs),
    #[command(about = "Generate Module:Ontology/data from the vendored copy")]
    Generate(GenerateArgs),
}

#[derive(Debug, Args)]
struct ExportArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "The packages/ontology directory of a clean remilia-sdk checkout"
    )]
    from: PathBuf,
}

#[derive(Debug, Args)]
struct GenerateArgs {
    #[arg(
        long,
        help = "Fail if the module differs from the vendored copy, without writing"
    )]
    check: bool,
}

pub(crate) fn run_ontology(runtime: &RuntimeOptions, args: OntologyArgs) -> Result<()> {
    let paths = resolve_runtime_paths(runtime)?;
    let export_path = paths.project_root.join(ontology::EXPORT_PATH);
    let module_path = paths.project_root.join(ontology::MODULE_PATH);
    match args.command {
        OntologySubcommand::Export(args) => {
            let export = ontology::export(&args.from)?;
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
                println!("{} matches the vendored copy", ontology::MODULE_PATH);
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
