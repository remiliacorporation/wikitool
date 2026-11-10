use std::error::Error;
use std::fmt;

use anyhow::{Context, Result, bail};
use mediawiki_protocol::{
    RemoteFileObservation, UploadOutcome, UploadRequest, UploadWarning, WikiUploadApi,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Serialize;
use sha1::{Digest, Sha1};

use crate::storage::{
    ensure_no_unresolved_remote_mutation, normalized_title_key, open_existing_sync_connection,
};
use crate::support::{compute_sha256, unix_timestamp};
use crate::{
    RemoteFileObservedState, RemoteMutationInspection, RemoteMutationOperation,
    RemoteMutationReconciliationReport, RemoteMutationReconciliationStatus, RemoteUploadAction,
    RemoteUploadInput, RemoteUploadPlan, RemoteUploadReport, RemoteUploadStatus, SyncProjectPaths,
    SyncTargetIdentity,
};

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteUploadError {
    MissingCredentials,
    MissingApplyPlanId,
    PlanMismatch {
        expected: String,
        supplied: String,
    },
    /// A different file already exists and the plan did not ask to replace it.
    ExistingFileRequiresReplace {
        target_api_url: String,
        title: String,
        observed_sha1: String,
        observed_timestamp: String,
    },
    /// MediaWiki answered with upload warnings and stored nothing.
    Warnings {
        mutation_id: i64,
        target_api_url: String,
        title: String,
        warnings: Vec<UploadWarning>,
    },
    NotApplied {
        mutation_id: i64,
        target_api_url: String,
        title: String,
        detail: String,
    },
    OutcomeAmbiguous {
        mutation_id: i64,
        target_api_url: String,
        title: String,
        detail: String,
    },
    ReconciliationRequired {
        mutation_id: i64,
        target_api_url: String,
        title: String,
        detail: String,
    },
}

impl fmt::Display for RemoteUploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCredentials => write!(
                formatter,
                "remote upload apply requires MediaWiki credentials"
            ),
            Self::MissingApplyPlanId => write!(
                formatter,
                "remote upload defaults to preview; apply requires the exact preview plan ID"
            ),
            Self::PlanMismatch { expected, supplied } => write!(
                formatter,
                "remote upload plan changed: current plan is {expected}, supplied plan was {supplied}; inspect the new preview before applying"
            ),
            Self::ExistingFileRequiresReplace {
                target_api_url,
                title,
                observed_sha1,
                observed_timestamp,
            } => write!(
                formatter,
                "{title} already exists on {target_api_url} with different content (SHA-1 {observed_sha1}, uploaded {observed_timestamp}); pass --replace to plan a new version over it"
            ),
            Self::Warnings {
                mutation_id,
                target_api_url,
                title,
                warnings,
            } => write!(
                formatter,
                "remote upload mutation {mutation_id} for {title} on {target_api_url} was not applied: MediaWiki answered with upload warning(s) {}; nothing was stored and the request was not repeated",
                warning_keys(warnings)
            ),
            Self::NotApplied {
                mutation_id,
                target_api_url,
                title,
                detail,
            } => write!(
                formatter,
                "remote upload mutation {mutation_id} for {title} on {target_api_url} was not applied: {detail}"
            ),
            Self::OutcomeAmbiguous {
                mutation_id,
                target_api_url,
                title,
                detail,
            } => write!(
                formatter,
                "remote upload mutation {mutation_id} for {title} on {target_api_url} has an ambiguous outcome: {detail}"
            ),
            Self::ReconciliationRequired {
                mutation_id,
                target_api_url,
                title,
                detail,
            } => write!(
                formatter,
                "remote upload mutation {mutation_id} for {title} on {target_api_url} requires reconciliation: {detail}"
            ),
        }
    }
}

impl Error for RemoteUploadError {}

#[derive(Serialize)]
struct UploadPlanIdentity<'a> {
    schema: &'static str,
    target_api_url: &'a str,
    canonical_title: &'a str,
    local_path: &'a str,
    file_size: u64,
    file_sha1: &'a str,
    action: RemoteUploadAction,
    replace_requested: bool,
    observed_state: RemoteFileObservedState,
    observed_sha1: Option<&'a str>,
    observed_size: Option<u64>,
    observed_timestamp: Option<&'a str>,
    duplicate_titles: &'a [String],
    text_sha256: Option<&'a str>,
    summary: &'a str,
}

struct LocalUpload {
    /// Target name without the namespace prefix, as sent to MediaWiki.
    filename: String,
    title: String,
    file_sha1: String,
    file_size: u64,
    detected_content: &'static str,
    text_sha256: Option<String>,
    summary: String,
}

struct UploadMutationIntent {
    mutation_id: i64,
    target_api_url: String,
    title: String,
    file_sha1: String,
    expected_sha1: Option<String>,
    expected_timestamp: Option<String>,
    phase: String,
    request_started_at_unix: Option<i64>,
    terminal_outcome: Option<String>,
}

/// Read local bytes and the target's current file state and return a stable
/// plan. This function never authenticates, writes MediaWiki, or touches the
/// durable sync store: uploaded files are not part of the page-sync mirror.
pub fn plan_remote_upload_with_api<A: WikiUploadApi>(
    target: &SyncTargetIdentity,
    api: &mut A,
    input: &RemoteUploadInput<'_>,
) -> Result<RemoteUploadPlan> {
    target.ensure_matches_api(api.target_api_url())?;
    let local = inspect_local_upload(input)?;
    plan_remote_upload_from_observation(target, api, input, &local)
}

/// Apply only the exact plan returned by [`plan_remote_upload_with_api`]. The
/// plan is recomputed from current bytes and current remote state after
/// authentication; the upload request is sent at most once.
pub fn apply_remote_upload_with_api<A: WikiUploadApi>(
    paths: &SyncProjectPaths,
    target: &SyncTargetIdentity,
    api: &mut A,
    input: &RemoteUploadInput<'_>,
    plan_id: Option<&str>,
    credentials: Option<(&str, &str)>,
) -> Result<RemoteUploadReport> {
    let supplied_plan = plan_id
        .filter(|value| !value.trim().is_empty())
        .ok_or(RemoteUploadError::MissingApplyPlanId)?;
    let (username, password) = credentials.ok_or(RemoteUploadError::MissingCredentials)?;

    target.ensure_matches_api(api.target_api_url())?;
    let local = inspect_local_upload(input)?;
    let connection = open_existing_sync_connection(paths, target)?;
    ensure_no_unresolved_remote_mutation(&connection, &local.title)?;
    api.login(username, password)?;

    let plan = plan_remote_upload_from_observation(target, api, input, &local)?;
    if plan.plan_id != supplied_plan {
        return Err(RemoteUploadError::PlanMismatch {
            expected: plan.plan_id,
            supplied: supplied_plan.to_string(),
        }
        .into());
    }
    if plan.action == RemoteUploadAction::AlreadyPresent {
        return Ok(RemoteUploadReport {
            status: RemoteUploadStatus::AlreadyPresent,
            target_api_url: target.api_url().to_string(),
            title: plan.canonical_title,
            mutation_id: None,
            file_sha1: plan.file_sha1,
            remote_timestamp: plan.observed_timestamp,
            detail: Some(
                "the target already stores exactly these bytes under this title; no upload request was sent"
                    .to_string(),
            ),
            request_count: api.request_count(),
        });
    }

    let intent = begin_upload_mutation(paths, &connection, target, &plan)?;
    if let Err(error) = mark_upload_request_started(&connection, &intent) {
        return Err(not_applied_error(
            &connection,
            &intent,
            format!("durable request-start marker failed; no request was sent: {error:#}"),
        ));
    }

    let outcome = api.upload_file(&UploadRequest {
        filename: &local.filename,
        bytes: input.bytes,
        text: input.text,
        comment: &local.summary,
        ignore_warnings: plan.action == RemoteUploadAction::Replace,
    });
    let response_timestamp = match outcome {
        Err(error) => {
            return Err(ambiguous_error(
                &connection,
                &intent,
                format!(
                    "upload request failed after its durable request-start marker: {error:#}; the request will not be retried"
                ),
            ));
        }
        Ok(UploadOutcome::Rejected { code, info }) if code.starts_with("internal_api_error") => {
            return Err(ambiguous_error(
                &connection,
                &intent,
                format!(
                    "MediaWiki failed internally while handling the upload [{code}]: {info}; the request will not be retried"
                ),
            ));
        }
        Ok(UploadOutcome::Rejected { code, info }) => {
            return Err(not_applied_error(
                &connection,
                &intent,
                format!("MediaWiki rejected the upload [{code}]: {info}"),
            ));
        }
        Ok(UploadOutcome::Warnings(warnings)) => {
            let detail = format!(
                "MediaWiki answered with upload warning(s) {} and stored nothing; the request was not repeated with ignorewarnings",
                warning_keys(&warnings)
            );
            return Err(
                match complete_upload_mutation(&connection, &intent, "not_applied", &detail, None) {
                    Ok(()) => RemoteUploadError::Warnings {
                        mutation_id: intent.mutation_id,
                        target_api_url: intent.target_api_url.clone(),
                        title: intent.title.clone(),
                        warnings,
                    }
                    .into(),
                    Err(error) => reconciliation_required_error(
                        &connection,
                        &intent,
                        format!(
                            "{detail}; additionally failed to persist terminal not_applied outcome: {error:#}"
                        ),
                    ),
                },
            );
        }
        Ok(UploadOutcome::Success {
            filename,
            timestamp,
            ..
        }) => {
            let response_title = format!("File:{filename}");
            if normalized_title_key(&response_title) != normalized_title_key(&intent.title) {
                return Err(reconciliation_required_error(
                    &connection,
                    &intent,
                    format!(
                        "MediaWiki reported success for {response_title:?}, not the planned title"
                    ),
                ));
            }
            timestamp
        }
    };

    // A success response is bound only after the stored bytes are re-observed.
    let current = match api.observe_file(&intent.title) {
        Ok(observation) => observation.versions.into_iter().next(),
        Err(error) => {
            return Err(reconciliation_required_error(
                &connection,
                &intent,
                format!(
                    "MediaWiki reported upload success, but the immediate file observation failed: {error:#}"
                ),
            ));
        }
    };
    let Some(current) =
        current.filter(|version| version.sha1.as_deref() == Some(&intent.file_sha1))
    else {
        return Err(reconciliation_required_error(
            &connection,
            &intent,
            format!(
                "MediaWiki reported upload success, but the current remote file does not expose SHA-1 {}",
                intent.file_sha1
            ),
        ));
    };
    let remote_timestamp = Some(current.timestamp);
    if let Err(error) = complete_upload_mutation(
        &connection,
        &intent,
        "uploaded",
        "upload response and the re-observed remote SHA-1 both match the planned file",
        remote_timestamp
            .as_deref()
            .or(response_timestamp.as_deref()),
    ) {
        return Err(reconciliation_required_error(
            &connection,
            &intent,
            format!("verified upload could not be recorded as terminal: {error:#}"),
        ));
    }
    Ok(RemoteUploadReport {
        status: RemoteUploadStatus::Uploaded,
        target_api_url: target.api_url().to_string(),
        title: plan.canonical_title,
        mutation_id: Some(intent.mutation_id),
        file_sha1: plan.file_sha1,
        remote_timestamp,
        detail: None,
        request_count: api.request_count(),
    })
}

/// Settle one durable upload receipt without ever replaying the write, by
/// comparing the remote file's SHA-1 history with the receipt.
pub(crate) fn reconcile_upload_mutation_with_api<A: WikiUploadApi>(
    paths: &SyncProjectPaths,
    target: &SyncTargetIdentity,
    api: &mut A,
    mutation_id: i64,
) -> Result<RemoteMutationReconciliationReport> {
    let connection = open_existing_sync_connection(paths, target)?;
    let intent = load_upload_mutation(&connection, mutation_id)?;
    if intent.target_api_url != target.api_url() {
        bail!(
            "upload mutation {mutation_id} belongs to target {}, not {}",
            intent.target_api_url,
            target.api_url()
        );
    }
    let report = |status, detail: String, request_count| RemoteMutationReconciliationReport {
        mutation_id,
        operation: RemoteMutationOperation::Upload,
        target_api_url: target.api_url().to_string(),
        title: intent.title.clone(),
        status,
        response_revision_id: None,
        deletion_log_id: None,
        detail,
        request_count,
    };

    if intent.phase == "resolved" {
        let (status, detail) = match intent.terminal_outcome.as_deref() {
            Some("uploaded") => (
                RemoteMutationReconciliationStatus::Applied,
                "upload mutation was already verified as applied",
            ),
            Some("not_applied") => (
                RemoteMutationReconciliationStatus::NotApplied,
                "upload mutation was already resolved as not applied",
            ),
            Some("applied_then_changed") => (
                RemoteMutationReconciliationStatus::AppliedThenChanged,
                "upload mutation was already resolved as applied then changed",
            ),
            other => {
                bail!("upload mutation {mutation_id} has unsupported terminal outcome {other:?}")
            }
        };
        return Ok(report(status, detail.to_string(), api.request_count()));
    }
    if intent.request_started_at_unix.is_none() {
        let detail = "the durable upload intent was never marked request-started; no MediaWiki upload request was issued";
        complete_upload_mutation(&connection, &intent, "not_applied", detail, None)?;
        return Ok(report(
            RemoteMutationReconciliationStatus::NotApplied,
            detail.to_string(),
            api.request_count(),
        ));
    }

    let observation = observe_exact_file(api, &intent.title)?;
    let current = observation.versions.first();
    if let Some(current) = current
        && current.sha1.as_deref() == Some(&intent.file_sha1)
    {
        let detail = format!(
            "the current remote file version has the receipt's SHA-1 {}",
            intent.file_sha1
        );
        complete_upload_mutation(
            &connection,
            &intent,
            "uploaded",
            &detail,
            Some(&current.timestamp),
        )?;
        return Ok(report(
            RemoteMutationReconciliationStatus::Applied,
            detail,
            api.request_count(),
        ));
    }

    let unchanged = match (current, intent.expected_sha1.as_deref()) {
        (None, None) => true,
        (Some(current), Some(expected)) => {
            current.sha1.as_deref() == Some(expected)
                && Some(current.timestamp.as_str()) == intent.expected_timestamp.as_deref()
        }
        _ => false,
    };
    if unchanged {
        let detail = match &intent.expected_sha1 {
            Some(expected) => format!(
                "the remote file is still the exact planned prior version (SHA-1 {expected}); the upload did not apply"
            ),
            None => {
                "the remote file is still absent, as planned; the upload did not apply".to_string()
            }
        };
        complete_upload_mutation(&connection, &intent, "not_applied", &detail, None)?;
        return Ok(report(
            RemoteMutationReconciliationStatus::NotApplied,
            detail,
            api.request_count(),
        ));
    }

    // MediaWiki timestamps are fixed-width UTC, so they order lexicographically.
    let superseded = observation.versions.iter().skip(1).find(|version| {
        version.sha1.as_deref() == Some(&intent.file_sha1)
            && intent
                .expected_timestamp
                .as_deref()
                .is_none_or(|expected| version.timestamp.as_str() > expected)
    });
    if let Some(version) = superseded {
        let detail = format!(
            "the version uploaded at {} has the receipt's SHA-1 {}, but a newer version is now current",
            version.timestamp, intent.file_sha1
        );
        complete_upload_mutation(
            &connection,
            &intent,
            "applied_then_changed",
            &detail,
            Some(&version.timestamp),
        )?;
        return Ok(report(
            RemoteMutationReconciliationStatus::AppliedThenChanged,
            detail,
            api.request_count(),
        ));
    }

    let hidden_history = !observation.history_complete
        || observation
            .versions
            .iter()
            .any(|version| version.sha1.is_none());
    let (phase, status, detail) = if hidden_history {
        (
            "outcome_ambiguous",
            RemoteMutationReconciliationStatus::StillAmbiguous,
            format!(
                "the visible remote history does not contain SHA-1 {}, but hidden or truncated file versions prevent proving whether this upload applied",
                intent.file_sha1
            ),
        )
    } else {
        (
            "reconciliation_required",
            RemoteMutationReconciliationStatus::ReconciliationRequired,
            format!(
                "the remote file matches neither the planned prior state nor the receipt's SHA-1 {}; it changed independently and operator review is required",
                intent.file_sha1
            ),
        )
    };
    mark_upload_mutation_unresolved(&connection, &intent, phase, &detail)?;
    Ok(report(status, detail, api.request_count()))
}

pub(crate) fn load_upload_inspections(
    connection: &Connection,
    unresolved_only: bool,
) -> Result<Vec<RemoteMutationInspection>> {
    let where_clause = if unresolved_only {
        " WHERE phase NOT IN ('state_advanced', 'resolved')"
    } else {
        ""
    };
    let sql = format!("{UPLOAD_INSPECTION_COLUMNS} FROM sync_upload_mutations{where_clause}");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], decode_upload_inspection)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to decode upload mutation list")
}

pub(crate) fn load_upload_inspection(
    connection: &Connection,
    mutation_id: i64,
) -> Result<RemoteMutationInspection> {
    connection
        .query_row(
            &format!(
                "{UPLOAD_INSPECTION_COLUMNS} FROM sync_upload_mutations WHERE mutation_id = ?1"
            ),
            [mutation_id],
            decode_upload_inspection,
        )
        .optional()?
        .with_context(|| format!("upload mutation {mutation_id} does not exist"))
}

const UPLOAD_INSPECTION_COLUMNS: &str =
    "SELECT mutation_id, target_api_url, title, local_path, phase,
        request_started_at_unix, terminal_outcome, file_sha1, detail";

fn decode_upload_inspection(row: &rusqlite::Row<'_>) -> rusqlite::Result<RemoteMutationInspection> {
    Ok(RemoteMutationInspection {
        mutation_id: row.get(0)?,
        operation: RemoteMutationOperation::Upload,
        target_api_url: row.get(1)?,
        title: row.get(2)?,
        relative_path: Some(row.get(3)?),
        phase: row.get(4)?,
        request_started: row.get::<_, Option<i64>>(5)?.is_some(),
        terminal_outcome: row.get(6)?,
        expected_revision_id: None,
        response_revision_id: None,
        deletion_log_id: None,
        file_sha1: Some(row.get(7)?),
        local_effect_policy: None,
        local_effect_status: None,
        detail: row.get(8)?,
        closure: None,
    })
}

fn plan_remote_upload_from_observation<A: WikiUploadApi>(
    target: &SyncTargetIdentity,
    api: &mut A,
    input: &RemoteUploadInput<'_>,
    local: &LocalUpload,
) -> Result<RemoteUploadPlan> {
    let observation = observe_exact_file(api, &local.title)?;
    let current = observation.versions.first();
    let observed_sha1 = match current {
        Some(version) => Some(version.sha1.clone().with_context(|| {
            format!(
                "the current version of {} is hidden; its content cannot be compared",
                observation.title
            )
        })?),
        None => None,
    };
    let title_key = normalized_title_key(&observation.title);
    let mut duplicate_titles = api
        .find_files_by_sha1(&local.file_sha1)?
        .into_iter()
        .filter(|title| normalized_title_key(title) != title_key)
        .collect::<Vec<_>>();
    duplicate_titles.sort();
    duplicate_titles.dedup();

    let action = match (current, observed_sha1.as_deref()) {
        (Some(_), Some(sha1)) if sha1 == local.file_sha1 => RemoteUploadAction::AlreadyPresent,
        (Some(_), Some(_)) if input.replace => RemoteUploadAction::Replace,
        (Some(version), Some(sha1)) => {
            return Err(RemoteUploadError::ExistingFileRequiresReplace {
                target_api_url: target.api_url().to_string(),
                title: observation.title,
                observed_sha1: sha1.to_string(),
                observed_timestamp: version.timestamp.clone(),
            }
            .into());
        }
        _ => RemoteUploadAction::Upload,
    };
    if action == RemoteUploadAction::Upload && local.text_sha256.is_none() {
        bail!(
            "{} does not exist on {}; a new file requires description-page wikitext (--text-file)",
            observation.title,
            target.api_url()
        );
    }

    let mut notes = Vec::new();
    match action {
        RemoteUploadAction::AlreadyPresent => notes.push(
            "the target already stores exactly these bytes under this title; apply sends nothing"
                .to_string(),
        ),
        RemoteUploadAction::Replace if local.text_sha256.is_some() => notes.push(
            "MediaWiki uses description text only when an upload creates the file page; the existing page text stays unchanged"
                .to_string(),
        ),
        RemoteUploadAction::Upload if !duplicate_titles.is_empty() => notes.push(
            "identical content already exists under another title; MediaWiki will answer with a `duplicate` warning and store nothing"
                .to_string(),
        ),
        _ => {}
    }

    let observed_state = if current.is_some() {
        RemoteFileObservedState::Present
    } else {
        RemoteFileObservedState::Missing
    };
    let observed_size = current.map(|version| version.size);
    let observed_timestamp = current.map(|version| version.timestamp.clone());
    let identity = UploadPlanIdentity {
        schema: "wikitool.remote-upload-plan.v1",
        target_api_url: target.api_url(),
        canonical_title: &observation.title,
        local_path: input.local_path,
        file_size: local.file_size,
        file_sha1: &local.file_sha1,
        action,
        replace_requested: input.replace,
        observed_state,
        observed_sha1: observed_sha1.as_deref(),
        observed_size,
        observed_timestamp: observed_timestamp.as_deref(),
        duplicate_titles: &duplicate_titles,
        text_sha256: local.text_sha256.as_deref(),
        summary: &local.summary,
    };
    let canonical = serde_json::to_string(&identity).context("failed to encode upload plan")?;
    Ok(RemoteUploadPlan {
        plan_id: compute_sha256(&canonical),
        target_api_url: target.api_url().to_string(),
        canonical_title: observation.title,
        local_path: input.local_path.to_string(),
        file_size: local.file_size,
        file_sha1: local.file_sha1.clone(),
        detected_content: local.detected_content.to_string(),
        action,
        replace_requested: input.replace,
        observed_state,
        observed_sha1,
        observed_size,
        observed_timestamp,
        duplicate_titles,
        text_sha256: local.text_sha256.clone(),
        summary: local.summary.clone(),
        notes,
        request_count: api.request_count(),
    })
}

fn observe_exact_file<A: WikiUploadApi>(api: &mut A, title: &str) -> Result<RemoteFileObservation> {
    let observation = api.observe_file(title)?;
    if normalized_title_key(&observation.title) != normalized_title_key(title) {
        bail!(
            "MediaWiki returned title {:?} for exact file observation {title:?}",
            observation.title
        );
    }
    Ok(observation)
}

fn inspect_local_upload(input: &RemoteUploadInput<'_>) -> Result<LocalUpload> {
    let filename = canonical_file_name(input.filename)?;
    let summary = input.summary.trim();
    if summary.is_empty() {
        bail!("remote upload requires a non-empty summary");
    }
    if input.bytes.is_empty() {
        bail!("{} is empty; refusing to upload it", input.local_path);
    }
    let (_, extension) = filename
        .rsplit_once('.')
        .expect("canonical file names carry an extension");
    let expected = media_kind_for_extension(extension).with_context(|| {
        format!(
            "unsupported upload extension {extension:?}; supported: jpg, jpeg, jpe, png, gif, webp, pdf"
        )
    })?;
    let detected = detect_media_kind(input.bytes);
    if detected != Some(expected) {
        bail!(
            "{} does not look like {expected} content (detected {}); the target name {filename:?} would mislabel it",
            input.local_path,
            detected.unwrap_or("an unrecognized format")
        );
    }
    if input.text.is_some_and(|text| text.trim().is_empty()) {
        bail!("description-page wikitext must not be empty");
    }
    let mut hasher = Sha1::new();
    hasher.update(input.bytes);
    let file_sha1 = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(LocalUpload {
        title: format!("File:{filename}"),
        filename,
        file_sha1,
        file_size: u64::try_from(input.bytes.len()).context("file size does not fit into u64")?,
        detected_content: expected,
        text_sha256: input.text.map(compute_sha256),
        summary: summary.to_string(),
    })
}

/// Canonicalize a target file name the way MediaWiki stores it: underscores
/// and whitespace runs become single spaces and the first character is
/// upper-cased. The `File:` prefix is not accepted here.
fn canonical_file_name(filename: &str) -> Result<String> {
    let collapsed = filename
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.is_empty() {
        bail!("remote upload requires a non-empty target file name");
    }
    if let Some(invalid) = collapsed.chars().find(|ch| {
        ch.is_control()
            || matches!(
                ch,
                ':' | '/' | '\\' | '#' | '<' | '>' | '[' | ']' | '|' | '{' | '}'
            )
    }) {
        bail!(
            "target file name {filename:?} contains {invalid:?}; pass the bare name without a namespace prefix or path"
        );
    }
    match collapsed.rsplit_once('.') {
        Some((stem, extension)) if !stem.trim().is_empty() && !extension.is_empty() => {}
        _ => bail!("target file name {filename:?} needs a base name and an extension"),
    }
    let mut chars = collapsed.chars();
    let first = chars.next().expect("non-empty file name");
    Ok(first.to_uppercase().chain(chars).collect())
}

fn media_kind_for_extension(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "jpe" => Some("jpeg"),
        "png" => Some("png"),
        "gif" => Some("gif"),
        "webp" => Some("webp"),
        "pdf" => Some("pdf"),
        _ => None,
    }
}

fn detect_media_kind(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpeg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"%PDF-") {
        Some("pdf")
    } else {
        None
    }
}

fn warning_keys(warnings: &[UploadWarning]) -> String {
    warnings
        .iter()
        .map(|warning| warning.key.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn begin_upload_mutation(
    paths: &SyncProjectPaths,
    connection: &Connection,
    target: &SyncTargetIdentity,
    plan: &RemoteUploadPlan,
) -> Result<UploadMutationIntent> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)
        .with_context(|| {
            format!(
                "failed to begin durable upload intent for {}",
                plan.canonical_title
            )
        })?;
    crate::store::bind_sync_target_for_state_write(paths, &transaction, target)?;
    ensure_no_unresolved_remote_mutation(&transaction, &plan.canonical_title)?;
    let now = i64::try_from(unix_timestamp()?).context("timestamp does not fit into i64")?;
    transaction
        .execute(
            "INSERT INTO sync_upload_mutations (
                target_api_url, title, local_path, file_sha1, file_size, text_sha256,
                summary, replace_existing, expected_sha1, expected_timestamp, plan_id,
                phase, created_at_unix, updated_at_unix
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'intent_persisted', ?12, ?12
             )",
            params![
                target.api_url(),
                plan.canonical_title,
                plan.local_path,
                plan.file_sha1,
                i64::try_from(plan.file_size).context("file size does not fit into i64")?,
                plan.text_sha256,
                plan.summary,
                i64::from(plan.action == RemoteUploadAction::Replace),
                plan.observed_sha1,
                plan.observed_timestamp,
                plan.plan_id,
                now,
            ],
        )
        .with_context(|| {
            format!(
                "failed to persist upload intent for {}",
                plan.canonical_title
            )
        })?;
    let mutation_id = transaction.last_insert_rowid();
    transaction.commit().with_context(|| {
        format!(
            "failed to commit upload intent for {}",
            plan.canonical_title
        )
    })?;
    Ok(UploadMutationIntent {
        mutation_id,
        target_api_url: target.api_url().to_string(),
        title: plan.canonical_title.clone(),
        file_sha1: plan.file_sha1.clone(),
        expected_sha1: plan.observed_sha1.clone(),
        expected_timestamp: plan.observed_timestamp.clone(),
        phase: "intent_persisted".to_string(),
        request_started_at_unix: None,
        terminal_outcome: None,
    })
}

fn load_upload_mutation(connection: &Connection, mutation_id: i64) -> Result<UploadMutationIntent> {
    connection
        .query_row(
            "SELECT mutation_id, target_api_url, title, file_sha1, expected_sha1,
                    expected_timestamp, phase, request_started_at_unix, terminal_outcome
             FROM sync_upload_mutations WHERE mutation_id = ?1",
            [mutation_id],
            |row| {
                Ok(UploadMutationIntent {
                    mutation_id: row.get(0)?,
                    target_api_url: row.get(1)?,
                    title: row.get(2)?,
                    file_sha1: row.get(3)?,
                    expected_sha1: row.get(4)?,
                    expected_timestamp: row.get(5)?,
                    phase: row.get(6)?,
                    request_started_at_unix: row.get(7)?,
                    terminal_outcome: row.get(8)?,
                })
            },
        )
        .optional()?
        .with_context(|| format!("upload mutation {mutation_id} does not exist"))
}

fn mark_upload_request_started(
    connection: &Connection,
    intent: &UploadMutationIntent,
) -> Result<()> {
    let now = i64::try_from(unix_timestamp()?).context("timestamp does not fit into i64")?;
    let changed = connection
        .execute(
            "UPDATE sync_upload_mutations
             SET request_started_at_unix = ?1, updated_at_unix = ?1
             WHERE mutation_id = ?2 AND phase = 'intent_persisted'
               AND request_started_at_unix IS NULL",
            params![now, intent.mutation_id],
        )
        .with_context(|| {
            format!(
                "failed to mark upload request {} started",
                intent.mutation_id
            )
        })?;
    if changed != 1 {
        bail!(
            "upload mutation {} for {} was not ready to start its request",
            intent.mutation_id,
            intent.title
        );
    }
    Ok(())
}

fn complete_upload_mutation(
    connection: &Connection,
    intent: &UploadMutationIntent,
    terminal_outcome: &'static str,
    detail: &str,
    response_timestamp: Option<&str>,
) -> Result<()> {
    let now = i64::try_from(unix_timestamp()?).context("timestamp does not fit into i64")?;
    let changed = connection
        .execute(
            "UPDATE sync_upload_mutations
             SET phase = 'resolved', terminal_outcome = ?1, detail = ?2,
                 response_timestamp = ?3, updated_at_unix = ?4
             WHERE mutation_id = ?5 AND phase NOT IN ('state_advanced', 'resolved')",
            params![
                terminal_outcome,
                detail,
                response_timestamp,
                now,
                intent.mutation_id
            ],
        )
        .with_context(|| format!("failed to complete upload mutation {}", intent.mutation_id))?;
    if changed != 1 {
        bail!("upload mutation {} is already terminal", intent.mutation_id);
    }
    Ok(())
}

fn mark_upload_mutation_unresolved(
    connection: &Connection,
    intent: &UploadMutationIntent,
    phase: &'static str,
    detail: &str,
) -> Result<()> {
    if !matches!(phase, "outcome_ambiguous" | "reconciliation_required") {
        bail!("invalid unresolved upload-mutation phase {phase}");
    }
    let now = i64::try_from(unix_timestamp()?).context("timestamp does not fit into i64")?;
    let changed = connection
        .execute(
            "UPDATE sync_upload_mutations
             SET phase = ?1, detail = ?2, updated_at_unix = ?3
             WHERE mutation_id = ?4 AND phase NOT IN ('state_advanced', 'resolved')",
            params![phase, detail, now, intent.mutation_id],
        )
        .with_context(|| {
            format!(
                "failed to mark upload mutation {} unresolved",
                intent.mutation_id
            )
        })?;
    if changed != 1 {
        bail!(
            "upload mutation {} for {} is no longer unresolved",
            intent.mutation_id,
            intent.title
        );
    }
    Ok(())
}

fn not_applied_error(
    connection: &Connection,
    intent: &UploadMutationIntent,
    detail: String,
) -> anyhow::Error {
    match complete_upload_mutation(connection, intent, "not_applied", &detail, None) {
        Ok(()) => RemoteUploadError::NotApplied {
            mutation_id: intent.mutation_id,
            target_api_url: intent.target_api_url.clone(),
            title: intent.title.clone(),
            detail,
        }
        .into(),
        Err(error) => reconciliation_required_error(
            connection,
            intent,
            format!(
                "{detail}; additionally failed to persist terminal not_applied outcome: {error:#}"
            ),
        ),
    }
}

fn ambiguous_error(
    connection: &Connection,
    intent: &UploadMutationIntent,
    detail: String,
) -> anyhow::Error {
    let persistence_error =
        mark_upload_mutation_unresolved(connection, intent, "outcome_ambiguous", &detail).err();
    RemoteUploadError::OutcomeAmbiguous {
        mutation_id: intent.mutation_id,
        target_api_url: intent.target_api_url.clone(),
        title: intent.title.clone(),
        detail: append_persistence_error(detail, persistence_error),
    }
    .into()
}

fn reconciliation_required_error(
    connection: &Connection,
    intent: &UploadMutationIntent,
    detail: String,
) -> anyhow::Error {
    let persistence_error =
        mark_upload_mutation_unresolved(connection, intent, "reconciliation_required", &detail)
            .err();
    RemoteUploadError::ReconciliationRequired {
        mutation_id: intent.mutation_id,
        target_api_url: intent.target_api_url.clone(),
        title: intent.title.clone(),
        detail: append_persistence_error(detail, persistence_error),
    }
    .into()
}

fn append_persistence_error(detail: String, error: Option<anyhow::Error>) -> String {
    match error {
        Some(error) => {
            format!("{detail}; additionally failed to persist mutation state: {error:#}")
        }
        None => detail,
    }
}
