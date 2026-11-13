use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;
use wikitool_core::article_lint::{
    ArticleFileLintReport, lint_article_file, lint_article_file_with_resources,
    load_article_lint_resources,
};

use crate::cli_support::{normalize_path, resolve_runtime_paths};
use crate::{LOCAL_DB_POLICY_MESSAGE, RuntimeOptions};

use super::output::{flag, print_article_target_selection, print_report};
use super::selection::{
    ArticleTargetSelection, article_selection_from_args, resolve_article_targets,
    single_file_lint_title,
};
use super::*;
#[derive(Debug, Serialize)]
struct ArticleLintBatchReport {
    project_root: String,
    strict: bool,
    selection: ArticleTargetSelection,
    target_count: usize,
    total_errors: usize,
    total_warnings: usize,
    total_suggestions: usize,
    reports: Vec<ArticleFileLintReport>,
}

pub(super) fn run_article_lint(runtime: &RuntimeOptions, args: ArticleLintArgs) -> Result<()> {
    let paths = resolve_runtime_paths(runtime)?;
    if let Some(title_override) = single_file_lint_title(
        &paths,
        args.path.as_deref(),
        &args.titles,
        &args.paths,
        args.titles_file.as_ref(),
        args.changed,
    ) {
        let file_report = lint_article_file(
            &paths,
            args.path.as_deref().expect("single path"),
            title_override,
        )?;
        let report = &file_report.report;

        if args.format.is_json() {
            println!("{}", serde_json::to_string_pretty(&file_report)?);
        } else {
            println!("article lint");
            println!("project_root: {}", normalize_path(&paths.project_root));
            if let Some(header) = &file_report.export_header {
                println!("export_header.title: {}", header.title);
                println!(
                    "export_header.source_url: {}",
                    header.source_url.as_deref().unwrap_or("<none>")
                );
                println!(
                    "export_header.revision_id: {}",
                    header.revision_id.as_deref().unwrap_or("<none>")
                );
                println!("export_header.line_count: {}", header.line_count);
                println!("file_sha256: {}", file_report.file_sha256);
            }
            print_report(report);
            println!("policy: {LOCAL_DB_POLICY_MESSAGE}");
            if runtime.diagnostics {
                println!("\n[diagnostics]\n{}", paths.diagnostics());
            }
        }

        if report.errors > 0 || (args.strict && report.warnings > 0) {
            bail!(
                "article lint found {} error(s), {} warning(s), and {} suggestion(s)",
                report.errors,
                report.warnings,
                report.suggestions
            );
        }
        return Ok(());
    }

    let selection = article_selection_from_args(
        &args.titles,
        &args.paths,
        args.titles_file.as_ref(),
        args.changed,
    )?;
    let target_paths = resolve_article_targets(&paths, args.path.as_deref(), &selection, false)?;
    let resources = load_article_lint_resources(&paths)?;
    let reports = target_paths
        .iter()
        .map(|relative_path| {
            lint_article_file_with_resources(&paths, Path::new(relative_path), None, &resources)
        })
        .collect::<Result<Vec<_>>>()?;
    let total_errors = reports.iter().map(|file| file.report.errors).sum();
    let total_warnings = reports.iter().map(|file| file.report.warnings).sum();
    let total_suggestions = reports.iter().map(|file| file.report.suggestions).sum();
    let batch_report = ArticleLintBatchReport {
        project_root: normalize_path(&paths.project_root),
        strict: args.strict,
        selection,
        target_count: reports.len(),
        total_errors,
        total_warnings,
        total_suggestions,
        reports,
    };

    if args.format.is_json() {
        println!("{}", serde_json::to_string_pretty(&batch_report)?);
    } else {
        println!("article lint");
        println!("project_root: {}", normalize_path(&paths.project_root));
        println!("strict: {}", flag(batch_report.strict));
        print_article_target_selection(&batch_report.selection);
        println!("target_count: {}", batch_report.target_count);
        println!("total_errors: {}", batch_report.total_errors);
        println!("total_warnings: {}", batch_report.total_warnings);
        println!("total_suggestions: {}", batch_report.total_suggestions);
        if batch_report.reports.is_empty() {
            println!("reports: <none>");
        } else {
            for file in &batch_report.reports {
                println!();
                print_report(&file.report);
            }
        }
        println!("policy: {LOCAL_DB_POLICY_MESSAGE}");
        if runtime.diagnostics {
            println!("\n[diagnostics]\n{}", paths.diagnostics());
        }
    }

    if batch_report.total_errors > 0 || (args.strict && batch_report.total_warnings > 0) {
        bail!(
            "article lint found {} error(s), {} warning(s), and {} suggestion(s) across {} file(s)",
            batch_report.total_errors,
            batch_report.total_warnings,
            batch_report.total_suggestions,
            batch_report.target_count
        );
    }
    Ok(())
}
