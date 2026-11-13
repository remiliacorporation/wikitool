use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use wikitool_core::site::load_site_adapter;

use crate::RuntimeOptions;
use crate::cli_support::{normalize_path, resolve_runtime_paths};
use crate::embedded_skills::{EmbeddedProcedure, embedded_procedures};
use crate::skills::parse_skill_identity;

const ADAPTER_TOPIC_PREFIX: &str = "adapter:";

#[derive(Debug, Args)]
pub(crate) struct GuideArgs {
    #[arg(
        value_name = "TOPIC",
        help = "A skill (wikitool), one of its references (wikitool/sync-and-acceptance) or a site-adapter guidance document (adapter:NAME); bare `guide` prints the orientation and every topic"
    )]
    topic: Option<String>,
}

pub(crate) fn run_guide(runtime: &RuntimeOptions, args: GuideArgs) -> Result<()> {
    let procedures = embedded_procedures();
    let adapter = SelectedAdapterGuidance::load(runtime);
    let Some(topic) = args.topic.as_deref().map(str::trim) else {
        print!("{}", orientation(&procedures, &adapter)?);
        return Ok(());
    };
    if let Some(name) = topic.strip_prefix(ADAPTER_TOPIC_PREFIX) {
        let document = adapter.find(name)?;
        let bytes = fs::read(&document.absolute_path).with_context(|| {
            format!(
                "failed to read site-adapter guidance {}",
                normalize_path(&document.absolute_path)
            )
        })?;
        return write_exact(&bytes);
    }
    write_exact(find_procedure(&procedures, topic)?.bytes)
}

fn write_exact(bytes: &[u8]) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(bytes)?;
    stdout.flush()?;
    Ok(())
}

fn find_procedure<'a>(
    procedures: &'a [EmbeddedProcedure],
    topic: &str,
) -> Result<&'a EmbeddedProcedure> {
    let requested = topic.trim_end_matches(".md");
    if let Some(procedure) = procedures
        .iter()
        .find(|procedure| procedure.topic() == requested)
    {
        return Ok(procedure);
    }
    // A bare reference name is accepted when exactly one skill owns it.
    let owners = procedures
        .iter()
        .filter(|procedure| procedure.reference == Some(requested))
        .collect::<Vec<_>>();
    match owners.as_slice() {
        [only] => Ok(only),
        [] => bail!(
            "unknown guide topic `{topic}`; run `wikitool guide` for the topic list, or `wikitool commands WORDS` to find a command"
        ),
        several => bail!(
            "guide topic `{topic}` is ambiguous; name one of: {}",
            several
                .iter()
                .map(|procedure| procedure.topic())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn orientation(
    procedures: &[EmbeddedProcedure],
    adapter: &SelectedAdapterGuidance,
) -> Result<String> {
    let mut out = String::new();
    out.push_str(&format!(
        "Wikitool {}: MediaWiki retrieval, templates, mechanical checks and revision-bound sync.\n",
        env!("CARGO_PKG_VERSION")
    ));
    out.push_str(
        "No model runs inside Wikitool. The skill procedures below ship in this binary;\n\
         `wikitool guide TOPIC` prints one exactly as an installed skill holds it.\n\n",
    );

    out.push_str("Skills, chosen by the requested outcome:\n");
    let entrypoints = procedures
        .iter()
        .filter(|procedure| procedure.reference.is_none())
        .collect::<Vec<_>>();
    let width = entrypoints
        .iter()
        .map(|procedure| procedure.skill.len())
        .max()
        .unwrap_or(0);
    for procedure in &entrypoints {
        let source = std::str::from_utf8(procedure.bytes)
            .with_context(|| format!("embedded skill {} is not UTF-8", procedure.skill))?;
        let (_, description) = parse_skill_identity(source, Path::new(procedure.skill))?;
        out.push_str(&format!("  {:width$}  {description}\n", procedure.skill));
    }
    out.push('\n');

    out.push_str(SYNC_BASELINE_RULE);
    out.push_str(ACCEPTANCE_RULE);
    out.push('\n');

    out.push_str("Topics (`wikitool guide TOPIC`):\n");
    let width = procedures
        .iter()
        .map(|procedure| procedure.topic().len())
        .max()
        .unwrap_or(0);
    for procedure in procedures {
        out.push_str(&format!(
            "  {:width$}  {}\n",
            procedure.topic(),
            procedure.heading().unwrap_or("")
        ));
    }
    out.push('\n');
    out.push_str(&adapter.summary());
    out.push('\n');
    out.push_str(
        "Find a command by purpose: `wikitool commands WORDS`. Flags: `wikitool COMMAND --help`.\n",
    );
    Ok(out)
}

const SYNC_BASELINE_RULE: &str = "\
Sync baseline: `status`, `diff`, `review` and `push` need the global revision
  baseline from one successful `pull --full --all`; a scoped pull does not
  establish it. Read-only retrieval (`export`, `source`, `templates show`)
  never needs it.
";

const ACCEPTANCE_RULE: &str = "\
Acceptance: a changed non-redirect Main page needs a named human's acceptance
  of its exact bytes (`article accept`) before `article promote` or `push`.
  Never invent acceptance. Lint and review are mechanical; they do not
  certify sources or prose.
";

/// The guidance documents of the site adapter the current project selects, or
/// why none can be listed. Loading never fails `guide` itself: it works outside
/// any project.
enum SelectedAdapterGuidance {
    Loaded {
        adapter_id: String,
        documents: Vec<AdapterGuidanceDocument>,
    },
    Unavailable(String),
}

struct AdapterGuidanceDocument {
    name: String,
    relative_path: String,
    absolute_path: PathBuf,
}

impl SelectedAdapterGuidance {
    fn load(runtime: &RuntimeOptions) -> Self {
        match Self::try_load(runtime) {
            Ok(loaded) => loaded,
            Err(error) => Self::Unavailable(format!("{error:#}")),
        }
    }

    fn try_load(runtime: &RuntimeOptions) -> Result<Self> {
        let paths = resolve_runtime_paths(runtime)?;
        let adapter = load_site_adapter(&paths)?;
        // The first source document is the adapter policy; the rest are the
        // guidance documents it declares, as project-relative paths.
        let adapter_dir = adapter
            .source_documents
            .first()
            .and_then(|document| Path::new(&document.relative_path).parent())
            .map(normalize_path)
            .unwrap_or_default();
        let documents = adapter
            .source_documents
            .iter()
            .skip(1)
            .map(|document| {
                let within_adapter = document
                    .relative_path
                    .strip_prefix(&adapter_dir)
                    .map(|rest| rest.trim_start_matches('/'))
                    .filter(|rest| !adapter_dir.is_empty() && !rest.is_empty())
                    .unwrap_or(&document.relative_path);
                AdapterGuidanceDocument {
                    name: within_adapter
                        .strip_suffix(".md")
                        .unwrap_or(within_adapter)
                        .to_string(),
                    relative_path: document.relative_path.clone(),
                    absolute_path: paths.project_root.join(&document.relative_path),
                }
            })
            .collect();
        Ok(Self::Loaded {
            adapter_id: adapter.adapter_id,
            documents,
        })
    }

    fn summary(&self) -> String {
        match self {
            Self::Loaded {
                adapter_id,
                documents,
            } if documents.is_empty() => {
                format!("Site adapter {adapter_id}: no supplemental guidance documents.\n")
            }
            Self::Loaded {
                adapter_id,
                documents,
            } => {
                let mut out = format!(
                    "Site adapter {adapter_id} guidance (site policy; `wikitool guide adapter:NAME`):\n"
                );
                let width = documents
                    .iter()
                    .map(|document| document.name.len() + ADAPTER_TOPIC_PREFIX.len())
                    .max()
                    .unwrap_or(0);
                for document in documents {
                    out.push_str(&format!(
                        "  {:width$}  {}\n",
                        format!("{ADAPTER_TOPIC_PREFIX}{}", document.name),
                        document.relative_path
                    ));
                }
                out
            }
            Self::Unavailable(reason) => {
                format!("Site adapter: not available here ({reason}).\n")
            }
        }
    }

    fn find(&self, name: &str) -> Result<&AdapterGuidanceDocument> {
        match self {
            Self::Loaded {
                adapter_id,
                documents,
            } => {
                let requested = name.trim().trim_end_matches(".md");
                documents
                    .iter()
                    .find(|document| document.name == requested)
                    .with_context(|| {
                        if documents.is_empty() {
                            format!("site adapter {adapter_id} declares no guidance documents")
                        } else {
                            format!(
                                "site adapter {adapter_id} has no guidance document `{name}`; available: {}",
                                documents
                                    .iter()
                                    .map(|document| document.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        }
                    })
            }
            Self::Unavailable(reason) => {
                bail!("no site adapter guidance is available here: {reason}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topics_resolve_by_canonical_name_bare_reference_and_extension() {
        let procedures = embedded_procedures();
        assert_eq!(
            find_procedure(&procedures, "wikitool").unwrap().topic(),
            "wikitool"
        );
        assert_eq!(
            find_procedure(&procedures, "sync-and-acceptance")
                .unwrap()
                .topic(),
            "wikitool/sync-and-acceptance"
        );
        assert_eq!(
            find_procedure(&procedures, "wiki-writing/human-notes.md")
                .unwrap()
                .topic(),
            "wiki-writing/human-notes"
        );
        let ambiguous = find_procedure(&procedures, "frozen-packet")
            .unwrap_err()
            .to_string();
        assert!(ambiguous.contains("prose-review/frozen-packet"));
        assert!(ambiguous.contains("wiki-writing/frozen-packet"));
        assert!(find_procedure(&procedures, "nonsense").is_err());
    }
}
