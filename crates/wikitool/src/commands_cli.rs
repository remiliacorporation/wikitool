use anyhow::Result;
use clap::{Args, Command, CommandFactory};
use serde::Serialize;

use crate::Cli;
use crate::cli_support::OutputFormat;

#[derive(Debug, Args)]
pub(crate) struct CommandsArgs {
    #[arg(
        value_name = "WORDS",
        help = "Every word must match the command path, its description or its purpose; omit to list every command"
    )]
    words: Vec<String>,
    #[arg(
        long,
        value_name = "N",
        help = "Maximum commands to print; the report still counts every match"
    )]
    limit: Option<usize>,
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        value_name = "FORMAT",
        help = "Output format: text|json"
    )]
    format: OutputFormat,
}

/// Who a command is for. Agent commands are what an editing agent uses; operator
/// commands set up or maintain a project, its corpora or its distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Audience {
    Agent,
    Operator,
}

impl Audience {
    fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Operator => "operator",
        }
    }
}

use Audience::{Agent, Operator};

/// Every visible leaf command, its audience and the task words an agent might
/// search for that its description does not already contain. A test keeps this
/// table equal to the command tree.
const COMMAND_PURPOSES: &[(&str, Audience, &str)] = &[
    ("guide", Agent, "procedures rules skills how to orientation"),
    ("commands", Agent, "find search command purpose discover"),
    ("init", Operator, "create project setup first run target"),
    ("config show", Agent, "which wiki target endpoint api url"),
    (
        "pull",
        Agent,
        "download mirror fetch baseline sync revision",
    ),
    (
        "push",
        Agent,
        "publish save edit write live preview apply plan",
    ),
    ("diff", Agent, "compare local edits baseline unified"),
    (
        "status",
        Agent,
        "what changed modified new deleted conflicts baseline",
    ),
    (
        "validate",
        Agent,
        "red broken links orphans redirects uncategorized",
    ),
    ("review", Agent, "pre-publish check gate lint push preview"),
    ("module lint", Agent, "lua scribunto check"),
    (
        "export",
        Agent,
        "read one live page download wikitext markdown",
    ),
    ("ontology export", Agent, "vendor ontology classes"),
    ("ontology generate", Agent, "ontology data module"),
    ("delete", Agent, "remove page live"),
    ("upload", Agent, "media image file"),
    ("mutation list", Agent, "receipts writes ambiguous outcome"),
    ("mutation show", Agent, "receipt write ambiguous outcome"),
    (
        "mutation reconcile",
        Agent,
        "ambiguous write recovery receipt",
    ),
    ("mutation close", Agent, "closure unresolved receipt"),
    ("db stats", Operator, "database catalog readiness size"),
    ("db reset", Operator, "clear database catalog"),
    ("docs import", Operator, "mediawiki documentation corpus"),
    (
        "docs import-technical",
        Operator,
        "mediawiki documentation corpus",
    ),
    (
        "docs import-profile",
        Operator,
        "mediawiki documentation corpus",
    ),
    ("docs list", Operator, "mediawiki documentation corpora"),
    ("docs update", Operator, "mediawiki documentation corpora"),
    ("docs remove", Operator, "mediawiki documentation corpus"),
    ("docs search", Agent, "mediawiki manual documentation help"),
    ("docs context", Agent, "mediawiki manual documentation help"),
    (
        "docs symbols",
        Agent,
        "hooks config variables api parser functions",
    ),
    ("import cargo", Agent, "cargo table csv json rows pages"),
    ("import html-to-wikitext", Agent, "convert html wikitext"),
    ("catalog build", Agent, "index search local pages"),
    ("catalog warm", Agent, "index docs profile"),
    ("catalog status", Agent, "readiness stale index"),
    (
        "catalog contracts search",
        Agent,
        "template contract authoring",
    ),
    (
        "catalog contracts plan",
        Agent,
        "template contract authoring",
    ),
    ("catalog inspect stats", Agent, "index counts"),
    (
        "catalog inspect chunks",
        Agent,
        "search retrieve text passages context",
    ),
    (
        "catalog inspect backlinks",
        Agent,
        "what links here inbound",
    ),
    (
        "catalog inspect templates",
        Agent,
        "template usage transclusion where used",
    ),
    (
        "catalog inspect references summary",
        Agent,
        "citations refs",
    ),
    ("catalog inspect references list", Agent, "citations refs"),
    (
        "catalog inspect references duplicates",
        Agent,
        "citations refs",
    ),
    ("catalog inspect orphans", Agent, "unlinked pages"),
    (
        "catalog inspect empty-categories",
        Agent,
        "empty categories",
    ),
    (
        "catalog surface sync",
        Agent,
        "templates modules extensions assets available",
    ),
    (
        "catalog surface show",
        Agent,
        "templates modules extensions assets available",
    ),
    (
        "adapter inspect",
        Agent,
        "site policy guidance publication identity",
    ),
    ("interview init", Agent, "human knowledge notes ledger"),
    ("interview validate", Agent, "human knowledge notes ledger"),
    ("interview show", Agent, "human knowledge notes ledger"),
    ("interview audit", Agent, "human knowledge notes ledgers"),
    (
        "interview open-item add",
        Agent,
        "question follow-up ledger",
    ),
    (
        "interview open-item list",
        Agent,
        "question follow-up ledger",
    ),
    (
        "interview open-item update",
        Agent,
        "question follow-up ledger",
    ),
    (
        "source wiki-search",
        Agent,
        "page exists live nearmatch title",
    ),
    ("source fetch", Agent, "read web page url citation"),
    ("source archive", Agent, "mirror snapshot web page"),
    ("source discover", Agent, "feeds api sitemap"),
    (
        "source session import",
        Agent,
        "cookies login paywall access",
    ),
    ("source session list", Agent, "cookies login access"),
    ("source session show", Agent, "cookies login access"),
    ("source session clear", Agent, "cookies login access"),
    (
        "source session prune",
        Agent,
        "cookies login access expired",
    ),
    (
        "source mediawiki-templates",
        Agent,
        "template parameters remote live",
    ),
    (
        "wiki capabilities sync",
        Agent,
        "extensions namespaces siteinfo",
    ),
    (
        "wiki capabilities show",
        Agent,
        "extensions namespaces siteinfo",
    ),
    (
        "wiki capabilities remote",
        Agent,
        "extensions namespaces siteinfo",
    ),
    ("wiki cargo tables", Agent, "cargo query"),
    ("wiki cargo fields", Agent, "cargo query schema"),
    ("wiki cargo rows", Agent, "cargo query"),
    ("wiki cargo count", Agent, "cargo query"),
    ("wiki render-check", Agent, "rendered html preview parse"),
    ("templates catalog build", Agent, "template index"),
    ("templates show", Agent, "template parameters documentation"),
    ("templates examples", Agent, "template usage invocations"),
    ("templates closure", Agent, "template dependencies modules"),
    ("templates contract capture", Agent, "template interface"),
    ("templates contract check", Agent, "template interface"),
    (
        "templates contract render-check",
        Agent,
        "template interface",
    ),
    ("templates scaffold", Agent, "create template"),
    (
        "templates migration-plan",
        Agent,
        "rename parameters migrate usages",
    ),
    (
        "article scout",
        Agent,
        "new article topic research context existing page",
    ),
    (
        "article accept",
        Agent,
        "human acceptance approve publication decision",
    ),
    (
        "article changeset prepare",
        Agent,
        "multi article batch acceptance",
    ),
    (
        "article changeset accept",
        Agent,
        "multi article batch acceptance",
    ),
    ("article lint", Agent, "check draft errors warnings"),
    ("article fix", Agent, "autofix safe"),
    ("article promote", Agent, "move draft publish"),
    ("lsp generate-config", Operator, "editor language server"),
    ("lsp status", Operator, "editor language server"),
    ("lsp info", Operator, "editor language server"),
    (
        "companions",
        Operator,
        "contextmink papertiger release tools",
    ),
    ("skills inspect", Operator, "skills harness distribution"),
    ("skills setup-project", Operator, "skills install harness"),
    (
        "skills uninstall-project",
        Operator,
        "skills remove harness",
    ),
];

#[derive(Debug, Clone, Serialize)]
struct CommandEntry {
    path: String,
    audience: Audience,
    about: String,
    purpose: &'static str,
}

#[derive(Debug, Serialize)]
struct CommandsReport {
    schema_version: &'static str,
    words: Vec<String>,
    matching_count: usize,
    shown_count: usize,
    commands: Vec<CommandEntry>,
}

pub(crate) fn run_commands(args: CommandsArgs) -> Result<()> {
    let words = args
        .words
        .iter()
        .flat_map(|word| word.split_whitespace())
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let matching = command_entries()
        .into_iter()
        .filter(|entry| entry_matches(entry, &words))
        .collect::<Vec<_>>();
    let matching_count = matching.len();
    let shown = matching
        .into_iter()
        .take(args.limit.unwrap_or(usize::MAX))
        .collect::<Vec<_>>();

    if args.format.is_json() {
        println!(
            "{}",
            serde_json::to_string_pretty(&CommandsReport {
                schema_version: "wikitool_commands_v1",
                words,
                matching_count,
                shown_count: shown.len(),
                commands: shown,
            })?
        );
        return Ok(());
    }

    for entry in &shown {
        println!("{} [{}]", entry.path, entry.audience.as_str());
        println!("  {}", entry.about);
    }
    if matching_count == 0 {
        println!(
            "No command matches every word. Fewer or broader words help; `wikitool commands` lists every command."
        );
    } else {
        println!(
            "{} of {matching_count} matching commands. `wikitool COMMAND --help` lists arguments; `wikitool guide` has the procedures.",
            shown.len()
        );
    }
    Ok(())
}

fn entry_matches(entry: &CommandEntry, words: &[String]) -> bool {
    let haystack = format!(
        "{} {} {} {}",
        entry.path,
        entry.about,
        entry.purpose,
        entry.audience.as_str()
    )
    .to_lowercase();
    words.iter().all(|word| haystack.contains(word.as_str()))
}

fn command_entries() -> Vec<CommandEntry> {
    let mut leaves = Vec::new();
    collect_leaf_commands(&Cli::command(), &[], &mut leaves);
    leaves
        .into_iter()
        .map(|(path, about)| {
            let (audience, purpose) = COMMAND_PURPOSES
                .iter()
                .find(|(candidate, _, _)| *candidate == path)
                .map(|(_, audience, purpose)| (*audience, *purpose))
                .unwrap_or((Agent, ""));
            CommandEntry {
                about: about.unwrap_or_else(|| purpose.to_string()),
                path,
                audience,
                purpose,
            }
        })
        .collect()
}

fn collect_leaf_commands(
    command: &Command,
    prefix: &[&str],
    out: &mut Vec<(String, Option<String>)>,
) {
    let visible = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set() && subcommand.get_name() != "help")
        .collect::<Vec<_>>();
    if visible.is_empty() && !prefix.is_empty() {
        let about = command
            .get_about()
            .map(|about| about.to_string())
            .and_then(|about| about.lines().next().map(str::to_string))
            .filter(|about| !about.trim().is_empty());
        out.push((prefix.join(" "), about));
        return;
    }
    for subcommand in visible {
        let mut next = prefix.to_vec();
        next.push(subcommand.get_name());
        collect_leaf_commands(subcommand, &next, out);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_visible_leaf_command_has_exactly_one_purpose_entry() {
        let mut leaves = Vec::new();
        collect_leaf_commands(&Cli::command(), &[], &mut leaves);
        let tree = leaves
            .into_iter()
            .map(|(path, _)| path)
            .collect::<BTreeSet<_>>();
        let table = COMMAND_PURPOSES
            .iter()
            .map(|(path, _, _)| (*path).to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            table.len(),
            table.iter().collect::<BTreeSet<_>>().len(),
            "COMMAND_PURPOSES repeats a command"
        );
        let table = table.into_iter().collect::<BTreeSet<_>>();
        let unclassified = tree.difference(&table).collect::<Vec<_>>();
        let stale = table.difference(&tree).collect::<Vec<_>>();
        assert!(
            unclassified.is_empty() && stale.is_empty(),
            "classify new commands in COMMAND_PURPOSES; unclassified={unclassified:?} stale={stale:?}"
        );
    }

    #[test]
    fn every_listed_command_has_a_one_line_purpose() {
        for entry in command_entries() {
            assert!(
                !entry.about.trim().is_empty(),
                "{} has no purpose",
                entry.path
            );
            assert!(
                !entry.about.contains('\n'),
                "{} purpose spans lines",
                entry.path
            );
        }
    }

    #[test]
    fn words_match_paths_descriptions_purposes_and_audience() {
        let entries = command_entries();
        let find = |words: &[&str]| {
            let words = words
                .iter()
                .map(|word| word.to_string())
                .collect::<Vec<_>>();
            entries
                .iter()
                .filter(|entry| entry_matches(entry, &words))
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>()
        };
        assert!(find(&["page", "exists"]).contains(&"source wiki-search"));
        assert!(find(&["lint", "article"]).contains(&"article lint"));
        assert_eq!(find(&["operator", "database", "clear"]), vec!["db reset"]);
        assert!(find(&["agent", "database", "clear"]).is_empty());
    }
}
