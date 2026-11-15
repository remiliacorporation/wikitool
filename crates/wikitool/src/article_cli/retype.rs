use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use wikitool_core::filesystem::{
    relative_path_to_title, title_to_relative_path, validate_scoped_path,
};
use wikitool_core::ontology::retype::{
    RetypeApplyMode, RetypeReport, run_article_retype as run_retype,
};

use crate::RuntimeOptions;
use crate::cli_support::{normalize_path, path_is_under_directory, resolve_runtime_paths};

use super::{ArticleRetypeApplyArg, ArticleRetypeArgs};

pub(super) fn run_article_retype(runtime: &RuntimeOptions, args: ArticleRetypeArgs) -> Result<()> {
    let paths = resolve_runtime_paths(runtime)?;
    let titles: Option<Vec<String>> = match (&args.path, &args.titles_file) {
        (Some(_), Some(_)) => bail!("article retype takes a PATH or --titles-file, not both"),
        (Some(path), None) => Some(vec![title_for_path(&paths, path)?]),
        (None, Some(file)) => {
            let text = fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", normalize_path(file)))?;
            Some(
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect(),
            )
        }
        (None, None) => None,
    };
    let apply = match args.apply {
        ArticleRetypeApplyArg::None => RetypeApplyMode::None,
        ArticleRetypeApplyArg::Draft => RetypeApplyMode::Draft,
    };
    let report = run_retype(&paths, &args.plan, titles.as_deref(), apply)?;

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

/// The title a mirror page or draft file stands for. A draft is named like
/// the mirror file of its title.
fn title_for_path(paths: &wikitool_core::runtime::ResolvedPaths, path: &Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        paths.project_root.join(path)
    };
    validate_scoped_path(paths, &absolute)?;
    let relative_to_root = |absolute: &Path| -> Result<String> {
        Ok(normalize_path(
            absolute
                .strip_prefix(&paths.project_root)
                .with_context(|| {
                    format!(
                        "{} is not inside the project {}",
                        normalize_path(absolute),
                        normalize_path(&paths.project_root)
                    )
                })?,
        ))
    };
    if path_is_under_directory(&absolute, &paths.state_dir.join("drafts")) {
        let file_name = absolute
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .context("the draft path has no file name")?;
        let sample = title_to_relative_path(paths, "Retype", false)?;
        let folder = sample.rsplit_once('/').map_or("", |(folder, _)| folder);
        return relative_path_to_title(paths, &format!("{folder}/{file_name}"));
    }
    if path_is_under_directory(&absolute, &paths.wiki_content_dir) {
        return relative_path_to_title(paths, &relative_to_root(&absolute)?);
    }
    bail!(
        "article retype reads pages under wiki_content/ or .wikitool/drafts/: {}",
        normalize_path(path)
    )
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
