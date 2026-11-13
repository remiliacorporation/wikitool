use anyhow::{Result, bail};
use serde::Serialize;
use wikitool_core::runtime::{ensure_runtime_ready_for_sync, inspect_runtime};
use wikitool_core::sync::{
    RemoteUploadAction, RemoteUploadError, RemoteUploadInput, RemoteUploadReport, UploadWarning,
    apply_remote_upload_with_config, plan_remote_upload_with_config,
};
use wikitool_core::upload::read_local_upload_source;

use crate::RuntimeOptions;
use crate::cli_support::{normalize_path, resolve_runtime_with_config};

use super::UploadArgs;

#[derive(Serialize)]
struct UploadApplyOutput<'a> {
    success: bool,
    mode: &'static str,
    project_root: String,
    target_api_url: &'a str,
    title: &'a str,
    summary: &'a str,
    plan_id: &'a str,
    remote: &'a RemoteUploadReport,
}

#[derive(Serialize)]
struct UploadErrorOutput<'a> {
    success: bool,
    status: &'static str,
    target_api_url: Option<&'a str>,
    mutation_id: Option<i64>,
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<&'a [UploadWarning]>,
    detail: String,
    next_action: Option<UploadNextAction>,
}

#[derive(Serialize)]
struct UploadNextAction {
    command: &'static str,
    operation: &'static str,
    mutation_id: i64,
}

pub(crate) fn run_upload(runtime: &RuntimeOptions, args: UploadArgs) -> Result<()> {
    let (paths, config) = resolve_runtime_with_config(runtime)?;
    if args.filename.trim().is_empty() {
        bail!("upload requires a non-empty --filename");
    }
    if args.summary.trim().is_empty() {
        bail!("upload requires a non-empty --summary");
    }
    let source = read_local_upload_source(&args.file, args.text_file.as_deref())?;
    let input = RemoteUploadInput {
        filename: &args.filename,
        local_path: &source.local_path,
        bytes: &source.bytes,
        text: source.text.as_deref(),
        summary: &args.summary,
        replace: args.replace,
    };

    let Some(plan_id) = args.apply.as_deref() else {
        let plan = match plan_remote_upload_with_config(&input, &config) {
            Ok(plan) => plan,
            Err(error) => {
                if args.format.is_json() {
                    print_structured_upload_error(&error)?;
                }
                return Err(error);
            }
        };
        let applicable = plan.action != RemoteUploadAction::AlreadyPresent;
        if args.format.is_json() {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "mode": "plan",
                    "project_root": normalize_path(&paths.project_root),
                    "plan": plan,
                    "next_action": applicable.then(|| serde_json::json!({
                        "command": "wikitool upload",
                        "apply_plan_id": plan.plan_id,
                    })),
                }))?
            );
        } else {
            let optional = |value: Option<String>| value.unwrap_or_else(|| "<none>".to_string());
            println!("upload plan");
            println!("project_root: {}", normalize_path(&paths.project_root));
            println!("target_api_url: {}", plan.target_api_url);
            println!("title: {}", plan.canonical_title);
            println!("local_path: {}", plan.local_path);
            println!("file_size: {}", plan.file_size);
            println!("file_sha1: {}", plan.file_sha1);
            println!("detected_content: {}", plan.detected_content);
            println!("action: {:?}", plan.action);
            println!("replace_requested: {}", plan.replace_requested);
            println!("observed_state: {:?}", plan.observed_state);
            println!("observed_sha1: {}", optional(plan.observed_sha1.clone()));
            println!(
                "observed_size: {}",
                optional(plan.observed_size.map(|value| value.to_string()))
            );
            println!(
                "observed_timestamp: {}",
                optional(plan.observed_timestamp.clone())
            );
            println!(
                "duplicate_titles: {}",
                if plan.duplicate_titles.is_empty() {
                    "<none>".to_string()
                } else {
                    plan.duplicate_titles.join(" | ")
                }
            );
            println!("text_sha256: {}", optional(plan.text_sha256.clone()));
            println!("summary: {}", plan.summary);
            for note in &plan.notes {
                println!("note: {note}");
            }
            println!("plan_id: {}", plan.plan_id);
            if applicable {
                println!("apply: rerun with --apply {}", plan.plan_id);
            } else {
                println!("apply: nothing to upload");
            }
        }
        return Ok(());
    };

    let status = inspect_runtime(&paths)?;
    ensure_runtime_ready_for_sync(&paths, &status)?;
    let remote = match apply_remote_upload_with_config(&paths, &input, plan_id, &config) {
        Ok(report) => report,
        Err(error) => {
            if args.format.is_json() {
                print_structured_upload_error(&error)?;
            }
            return Err(error);
        }
    };

    if args.format.is_json() {
        println!(
            "{}",
            serde_json::to_string_pretty(&UploadApplyOutput {
                success: true,
                mode: "apply",
                project_root: normalize_path(&paths.project_root),
                target_api_url: &remote.target_api_url,
                title: &remote.title,
                summary: &args.summary,
                plan_id,
                remote: &remote,
            })?
        );
        return Ok(());
    }

    println!("upload apply");
    println!("project_root: {}", normalize_path(&paths.project_root));
    println!("target_api_url: {}", remote.target_api_url);
    println!("title: {}", remote.title);
    println!("status: {:?}", remote.status);
    println!(
        "mutation_id: {}",
        remote
            .mutation_id
            .map(|value| value.to_string())
            .as_deref()
            .unwrap_or("<none>")
    );
    println!("file_sha1: {}", remote.file_sha1);
    println!(
        "remote_timestamp: {}",
        remote.remote_timestamp.as_deref().unwrap_or("<none>")
    );
    if let Some(detail) = &remote.detail {
        println!("detail: {detail}");
    }
    Ok(())
}

fn print_structured_upload_error(error: &anyhow::Error) -> Result<()> {
    let reconcile = |mutation_id: i64| UploadNextAction {
        command: "wikitool mutation reconcile",
        operation: "upload",
        mutation_id,
    };
    let show = |mutation_id: i64| UploadNextAction {
        command: "wikitool mutation show",
        operation: "upload",
        mutation_id,
    };
    let output = match error.downcast_ref::<RemoteUploadError>() {
        None => UploadErrorOutput {
            success: false,
            status: "error",
            target_api_url: None,
            mutation_id: None,
            title: None,
            warnings: None,
            detail: format!("{error:#}"),
            next_action: None,
        },
        Some(remote_error) => {
            let (status, target_api_url, mutation_id, title, warnings, next_action) =
                match remote_error {
                    RemoteUploadError::MissingCredentials => {
                        ("missing_credentials", None, None, None, None, None)
                    }
                    RemoteUploadError::MissingApplyPlanId => {
                        ("missing_plan_id", None, None, None, None, None)
                    }
                    RemoteUploadError::PlanMismatch { .. } => {
                        ("plan_mismatch", None, None, None, None, None)
                    }
                    RemoteUploadError::ExistingFileRequiresReplace {
                        target_api_url,
                        title,
                        ..
                    } => (
                        "existing_file_requires_replace",
                        Some(target_api_url.as_str()),
                        None,
                        Some(title.as_str()),
                        None,
                        None,
                    ),
                    RemoteUploadError::Warnings {
                        mutation_id,
                        target_api_url,
                        title,
                        warnings,
                    } => (
                        "upload_warnings",
                        Some(target_api_url.as_str()),
                        Some(*mutation_id),
                        Some(title.as_str()),
                        Some(warnings.as_slice()),
                        Some(show(*mutation_id)),
                    ),
                    RemoteUploadError::NotApplied {
                        mutation_id,
                        target_api_url,
                        title,
                        ..
                    } => (
                        "not_applied",
                        Some(target_api_url.as_str()),
                        Some(*mutation_id),
                        Some(title.as_str()),
                        None,
                        Some(show(*mutation_id)),
                    ),
                    RemoteUploadError::OutcomeAmbiguous {
                        mutation_id,
                        target_api_url,
                        title,
                        ..
                    } => (
                        "outcome_ambiguous",
                        Some(target_api_url.as_str()),
                        Some(*mutation_id),
                        Some(title.as_str()),
                        None,
                        Some(reconcile(*mutation_id)),
                    ),
                    RemoteUploadError::ReconciliationRequired {
                        mutation_id,
                        target_api_url,
                        title,
                        ..
                    } => (
                        "reconciliation_required",
                        Some(target_api_url.as_str()),
                        Some(*mutation_id),
                        Some(title.as_str()),
                        None,
                        Some(reconcile(*mutation_id)),
                    ),
                };
            UploadErrorOutput {
                success: false,
                status,
                target_api_url,
                mutation_id,
                title,
                warnings,
                detail: remote_error.to_string(),
                next_action,
            }
        }
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
