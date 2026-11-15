use std::fs;

use anyhow::{Context, Result, bail};
use wikitool_core::ontology::retype::{
    RetypeApplyMode, RetypeReport, RetypeTargets, run_article_retype as run_retype,
};

use crate::RuntimeOptions;
use crate::cli_support::{normalize_path, resolve_runtime_paths};

use super::{ArticleRetypeApplyArg, ArticleRetypeArgs};

pub(super) fn run_article_retype(runtime: &RuntimeOptions, args: ArticleRetypeArgs) -> Result<()> {
    let paths = resolve_runtime_paths(runtime)?;
    let targets = match (args.path, &args.titles_file) {
        (Some(_), Some(_)) => bail!("article retype takes a PATH or --titles-file, not both"),
        // The exact file named, never another file of the same title.
        (Some(path), None) => RetypeTargets::File(path),
        (None, Some(file)) => {
            let text = fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", normalize_path(file)))?;
            RetypeTargets::Titles(titles_from_file(&text))
        }
        (None, None) => RetypeTargets::Plan,
    };
    let apply = match args.apply {
        ArticleRetypeApplyArg::None => RetypeApplyMode::None,
        ArticleRetypeApplyArg::Draft => RetypeApplyMode::Draft,
    };
    let report = run_retype(&paths, &args.plan, &targets, apply)?;

    if args.format.is_json() {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&normalize_path(&paths.project_root), &report);
    }
    if report.status != "complete" {
        bail!(
            "article retype cannot run: {}",
            report
                .reason
                .as_deref()
                .unwrap_or("the project is not ready")
        );
    }
    if report.refused_count > 0 {
        bail!(
            "article retype refused {} of {} page(s)",
            report.refused_count,
            report.page_count
        );
    }
    Ok(())
}

/// The titles of a titles file, one per line, without the byte order mark an
/// editor may start the file with.
fn titles_from_file(text: &str) -> Vec<String> {
    text.strip_prefix('\u{feff}')
        .unwrap_or(text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn print_report(project_root: &str, report: &RetypeReport) {
    println!("article retype");
    println!("project_root: {project_root}");
    println!("plan: {} (sha256 {})", report.plan_path, report.plan_sha256);
    println!("apply_mode: {}", report.apply_mode);
    if let Some(reason) = &report.reason {
        println!("status: {}", report.status);
        println!("reason: {reason}");
        return;
    }
    println!(
        "ontology_commit: {}",
        report.ontology_commit.as_deref().unwrap_or("<none>")
    );
    println!(
        "pages: {} (changed {}, unchanged {}, refused {}); edits: {}; drafts_written: {}",
        report.page_count,
        report.changed_count,
        report.unchanged_count,
        report.refused_count,
        report.edit_count,
        report.drafts_written
    );
    for page in &report.pages {
        println!();
        println!("page: {}", page.title);
        println!("status: {}", page.status);
        if let (Some(source), Some(kind)) = (&page.source_path, page.source_kind) {
            println!("source: {source} ({kind})");
        }
        if let Some(draft) = &page.draft_path {
            println!(
                "draft: {draft}{}",
                if page.draft_written { " (written)" } else { "" }
            );
        }
        for edit in &page.edits {
            let categories = if edit.categories.is_empty() {
                String::new()
            } else {
                format!(" [{}]", edit.categories.join(", "))
            };
            println!(
                "edit: {}{categories} bytes {}..{}",
                edit.kind, edit.start, edit.end
            );
        }
        for refusal in &page.refusals {
            println!("refused: {}: {}", refusal.code, refusal.message);
        }
        for note in &page.notes {
            println!("note: {note}");
        }
        if let Some(typing) = &page.typing {
            for finding in &typing.findings {
                println!("typing: {}: {}", finding.code, finding.message);
            }
        }
        if !page.diff.is_empty() {
            print!("{}", page.diff);
            if !page.diff.ends_with('\n') {
                println!();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::titles_from_file;

    #[test]
    fn a_titles_file_may_start_with_a_byte_order_mark() {
        assert_eq!(
            titles_from_file("\u{feff}Wojak\r\n\r\n Pepe \n"),
            vec!["Wojak".to_string(), "Pepe".to_string()]
        );
    }
}
