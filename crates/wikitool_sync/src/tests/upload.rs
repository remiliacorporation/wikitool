use std::collections::BTreeMap;
use std::fs;

use serde_json::json;
use sha1::{Digest, Sha1};
use tempfile::tempdir;

use super::{MockApi, authoritative_namespaces, default_target, paths, pull_from_remote_with_api};
use crate::{
    PullCoverage, PullOptions, RemoteFileObservation, RemoteFileObservedState, RemoteFileVersion,
    RemoteMutationOperation, RemoteMutationReconciliationStatus, RemoteUploadAction,
    RemoteUploadError, RemoteUploadInput, RemoteUploadStatus, SyncProjectPaths, UploadOutcome,
    UploadRequest, UploadWarning, WikiUploadApi, apply_remote_upload_with_api,
    list_remote_mutations, plan_remote_upload_with_api, reconcile_remote_mutation_with_api,
    show_remote_mutation,
};

const TITLE: &str = "File:Sample photo.jpg";
const JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x01,
];
const OTHER_JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x08, b'E', b'x', b'i', b'f', 0x02,
];
const THIRD_JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xDB, 0x03];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordedUpload {
    filename: String,
    sha1: String,
    text: Option<String>,
    comment: String,
    ignore_warnings: bool,
}

/// In-memory stand-in for MediaWiki's local file repository.
#[derive(Default)]
pub(super) struct UploadFake {
    /// Stored versions per canonical `File:` title, newest first.
    files: BTreeMap<String, Vec<RemoteFileVersion>>,
    /// Answer the next upload with this server response and store nothing.
    response: Option<UploadOutcome>,
    /// Fail the transport before the server stores the file.
    error_before_store: Option<String>,
    /// Store the file, then lose the response.
    error_after_store: Option<String>,
    requests: Vec<RecordedUpload>,
}

impl UploadFake {
    fn store(&mut self, title: &str, bytes: &[u8]) -> RemoteFileVersion {
        let versions = self.files.entry(title.to_string()).or_default();
        let version = RemoteFileVersion {
            sha1: Some(sha1_hex(bytes)),
            size: bytes.len() as u64,
            timestamp: format!("2026-11-10T00:00:{:02}Z", versions.len() + 1),
        };
        versions.insert(0, version.clone());
        version
    }
}

impl WikiUploadApi for MockApi {
    fn observe_file(&mut self, title: &str) -> anyhow::Result<RemoteFileObservation> {
        self.request_count += 1;
        let title = title.replace('_', " ");
        Ok(RemoteFileObservation {
            versions: self.upload.files.get(&title).cloned().unwrap_or_default(),
            title,
            history_complete: true,
        })
    }

    fn find_files_by_sha1(&mut self, sha1: &str) -> anyhow::Result<Vec<String>> {
        self.request_count += 1;
        Ok(self
            .upload
            .files
            .iter()
            .filter(|(_, versions)| {
                versions
                    .first()
                    .is_some_and(|version| version.sha1.as_deref() == Some(sha1))
            })
            .map(|(title, _)| title.clone())
            .collect())
    }

    fn upload_file(&mut self, request: &UploadRequest<'_>) -> anyhow::Result<UploadOutcome> {
        self.request_count += 1;
        if !self.logged_in {
            anyhow::bail!("upload attempted without authentication");
        }
        self.upload.requests.push(RecordedUpload {
            filename: request.filename.to_string(),
            sha1: sha1_hex(request.bytes),
            text: request.text.map(ToString::to_string),
            comment: request.comment.to_string(),
            ignore_warnings: request.ignore_warnings,
        });
        if let Some(message) = self.upload.error_before_store.take() {
            anyhow::bail!(message);
        }
        if let Some(response) = self.upload.response.take() {
            return Ok(response);
        }
        let version = self
            .upload
            .store(&format!("File:{}", request.filename), request.bytes);
        if let Some(message) = self.upload.error_after_store.take() {
            anyhow::bail!(message);
        }
        Ok(UploadOutcome::Success {
            filename: request.filename.replace(' ', "_"),
            sha1: version.sha1,
            timestamp: Some(version.timestamp),
        })
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn input(bytes: &'static [u8], replace: bool) -> RemoteUploadInput<'static> {
    RemoteUploadInput {
        filename: "sample_photo.jpg",
        local_path: "D:/media/sample.jpeg",
        bytes,
        text: Some("== Summary ==\nA sample photo."),
        summary: "Upload sample photo",
        replace,
    }
}

/// Upload receipts live in the established, target-bound sync store.
fn established_project() -> (tempfile::TempDir, SyncProjectPaths, MockApi) {
    let temp = tempdir().expect("tempdir");
    let project_root = temp.path().join("project");
    fs::create_dir_all(project_root.join("wiki_content")).expect("create wiki_content");
    let paths = paths(&project_root);
    let mut api = MockApi::default();
    pull_from_remote_with_api(
        &paths,
        &PullOptions {
            namespaces: authoritative_namespaces(),
            category: None,
            full: true,
            coverage: PullCoverage::GlobalAllNamespaces,
            overwrite_local: false,
        },
        &mut api,
    )
    .expect("establish empty baseline");
    (temp, paths, api)
}

fn plan_and_apply(
    paths: &SyncProjectPaths,
    api: &mut MockApi,
    input: &RemoteUploadInput<'_>,
) -> anyhow::Result<crate::RemoteUploadReport> {
    let plan = plan_remote_upload_with_api(&default_target(), api, input)?;
    apply_remote_upload_with_api(
        paths,
        &default_target(),
        api,
        input,
        Some(&plan.plan_id),
        Some(("bot", "pass")),
    )
}

fn upload_error(error: &anyhow::Error) -> &RemoteUploadError {
    error
        .downcast_ref::<RemoteUploadError>()
        .unwrap_or_else(|| panic!("expected a typed upload error, got {error:#}"))
}

fn unresolved_count(paths: &SyncProjectPaths) -> usize {
    list_remote_mutations(paths, &default_target(), true)
        .expect("unresolved list")
        .mutations
        .len()
}

#[test]
fn new_upload_is_verified_by_remote_sha1_and_recorded_outside_the_page_mirror() {
    let (_temp, paths, mut api) = established_project();
    let input = input(JPEG, false);

    let plan = plan_remote_upload_with_api(&default_target(), &mut api, &input).expect("plan");
    assert_eq!(plan.canonical_title, TITLE);
    assert_eq!(plan.action, RemoteUploadAction::Upload);
    assert_eq!(plan.observed_state, RemoteFileObservedState::Missing);
    assert_eq!(plan.file_sha1, sha1_hex(JPEG));
    assert_eq!(plan.file_size, JPEG.len() as u64);
    assert!(!api.logged_in, "planning must not authenticate");
    assert!(api.upload.requests.is_empty());

    let report = apply_remote_upload_with_api(
        &paths,
        &default_target(),
        &mut api,
        &input,
        Some(&plan.plan_id),
        Some(("bot", "pass")),
    )
    .expect("apply");
    assert_eq!(report.status, RemoteUploadStatus::Uploaded);
    assert_eq!(
        api.upload.requests,
        vec![RecordedUpload {
            filename: "Sample photo.jpg".to_string(),
            sha1: sha1_hex(JPEG),
            text: Some("== Summary ==\nA sample photo.".to_string()),
            comment: "Upload sample photo".to_string(),
            ignore_warnings: false,
        }]
    );

    let receipts = list_remote_mutations(&paths, &default_target(), false)
        .expect("all receipts")
        .mutations;
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].operation, RemoteMutationOperation::Upload);
    assert_eq!(receipts[0].title, TITLE);
    assert_eq!(receipts[0].phase, "resolved");
    assert_eq!(receipts[0].terminal_outcome.as_deref(), Some("uploaded"));
    assert_eq!(receipts[0].file_sha1, Some(sha1_hex(JPEG)));
    assert_eq!(unresolved_count(&paths), 0);
    assert_eq!(
        fs::read_dir(&paths.wiki_content_dir)
            .expect("wiki_content")
            .count(),
        0,
        "an upload must not write into the page mirror"
    );
}

#[test]
fn apply_rejects_local_and_remote_drift_from_the_reviewed_plan() {
    let (_temp, paths, mut api) = established_project();
    api.upload.store(TITLE, OTHER_JPEG);
    let reviewed = plan_remote_upload_with_api(&default_target(), &mut api, &input(JPEG, true))
        .expect("replace plan");
    assert_eq!(reviewed.action, RemoteUploadAction::Replace);

    let local_drift = apply_remote_upload_with_api(
        &paths,
        &default_target(),
        &mut api,
        &input(THIRD_JPEG, true),
        Some(&reviewed.plan_id),
        Some(("bot", "pass")),
    )
    .expect_err("changed local bytes must not reuse the reviewed plan");
    assert!(matches!(
        upload_error(&local_drift),
        RemoteUploadError::PlanMismatch { supplied, .. } if *supplied == reviewed.plan_id
    ));

    api.upload.store(TITLE, THIRD_JPEG);
    let remote_drift = apply_remote_upload_with_api(
        &paths,
        &default_target(),
        &mut api,
        &input(JPEG, true),
        Some(&reviewed.plan_id),
        Some(("bot", "pass")),
    )
    .expect_err("a new remote version must invalidate the reviewed plan");
    assert!(matches!(
        upload_error(&remote_drift),
        RemoteUploadError::PlanMismatch { .. }
    ));

    assert!(api.upload.requests.is_empty());
    assert!(
        list_remote_mutations(&paths, &default_target(), false)
            .expect("all receipts")
            .mutations
            .is_empty(),
        "a rejected plan must not leave a receipt"
    );
}

#[test]
fn existing_file_is_refused_at_plan_time_unless_replace_is_planned() {
    let (_temp, paths, mut api) = established_project();
    let existing = api.upload.store(TITLE, OTHER_JPEG);

    let refused = plan_remote_upload_with_api(&default_target(), &mut api, &input(JPEG, false))
        .expect_err("overwrite without --replace must be refused");
    assert_eq!(
        upload_error(&refused),
        &RemoteUploadError::ExistingFileRequiresReplace {
            target_api_url: default_target().api_url().to_string(),
            title: TITLE.to_string(),
            observed_sha1: existing.sha1.clone().expect("sha1"),
            observed_timestamp: existing.timestamp.clone(),
        }
    );

    let report = plan_and_apply(&paths, &mut api, &input(JPEG, true)).expect("replace");
    assert_eq!(report.status, RemoteUploadStatus::Uploaded);
    assert_eq!(api.upload.requests.len(), 1);
    assert!(
        api.upload.requests[0].ignore_warnings,
        "only a planned replacement may ignore upload warnings"
    );
    assert_eq!(
        api.upload.files[TITLE][0].sha1,
        Some(sha1_hex(JPEG)),
        "the new version is current"
    );
}

#[test]
fn identical_remote_file_is_reported_present_and_never_reuploaded() {
    let (_temp, paths, mut api) = established_project();
    api.upload.store(TITLE, JPEG);
    api.upload.store("File:Elsewhere.jpg", JPEG);

    let plan = plan_remote_upload_with_api(&default_target(), &mut api, &input(JPEG, false))
        .expect("plan");
    assert_eq!(plan.action, RemoteUploadAction::AlreadyPresent);
    assert_eq!(
        plan.duplicate_titles,
        vec!["File:Elsewhere.jpg".to_string()]
    );

    let report = plan_and_apply(&paths, &mut api, &input(JPEG, false)).expect("apply");
    assert_eq!(report.status, RemoteUploadStatus::AlreadyPresent);
    assert_eq!(report.mutation_id, None);
    assert!(api.upload.requests.is_empty());
}

#[test]
fn upload_warning_is_terminal_not_applied_and_never_retried_with_ignorewarnings() {
    let (_temp, paths, mut api) = established_project();
    api.upload.response = Some(UploadOutcome::Warnings(vec![
        UploadWarning {
            key: "badfilename".to_string(),
            value: json!("Sample_photo.jpg"),
        },
        UploadWarning {
            key: "duplicate".to_string(),
            value: json!(["Elsewhere.jpg"]),
        },
    ]));

    let error = plan_and_apply(&paths, &mut api, &input(JPEG, false))
        .expect_err("a warning response stores nothing");
    let RemoteUploadError::Warnings {
        mutation_id,
        warnings,
        ..
    } = upload_error(&error)
    else {
        panic!("expected upload warnings, got {error:#}");
    };
    assert_eq!(
        warnings
            .iter()
            .map(|warning| warning.key.as_str())
            .collect::<Vec<_>>(),
        vec!["badfilename", "duplicate"]
    );
    assert_eq!(api.upload.requests.len(), 1);
    assert!(!api.upload.requests[0].ignore_warnings);
    assert!(!api.upload.files.contains_key(TITLE));

    let receipt = show_remote_mutation(
        &paths,
        &default_target(),
        RemoteMutationOperation::Upload,
        *mutation_id,
    )
    .expect("terminal receipt");
    assert_eq!(receipt.phase, "resolved");
    assert_eq!(receipt.terminal_outcome.as_deref(), Some("not_applied"));
    assert_eq!(unresolved_count(&paths), 0);
}

#[test]
fn server_rejection_is_not_applied_but_internal_failure_stays_ambiguous() {
    let (_temp, paths, mut api) = established_project();
    api.upload.response = Some(UploadOutcome::Rejected {
        code: "permissiondenied".to_string(),
        info: "not allowed".to_string(),
    });
    let rejected =
        plan_and_apply(&paths, &mut api, &input(JPEG, false)).expect_err("permission error");
    assert!(matches!(
        upload_error(&rejected),
        RemoteUploadError::NotApplied { .. }
    ));
    assert_eq!(unresolved_count(&paths), 0);

    api.upload.response = Some(UploadOutcome::Rejected {
        code: "internal_api_error_DBQueryError".to_string(),
        info: "database error".to_string(),
    });
    let internal =
        plan_and_apply(&paths, &mut api, &input(JPEG, false)).expect_err("internal server failure");
    assert!(matches!(
        upload_error(&internal),
        RemoteUploadError::OutcomeAmbiguous { .. }
    ));
    assert_eq!(unresolved_count(&paths), 1);
}

#[test]
fn lost_response_is_ambiguous_blocks_replay_and_reconciles_by_remote_sha1() {
    let (_temp, paths, mut api) = established_project();
    api.upload.error_after_store = Some("connection reset".to_string());

    let error = plan_and_apply(&paths, &mut api, &input(JPEG, false))
        .expect_err("lost response is ambiguous");
    let RemoteUploadError::OutcomeAmbiguous { mutation_id, .. } = upload_error(&error) else {
        panic!("expected an ambiguous outcome, got {error:#}");
    };
    let mutation_id = *mutation_id;
    assert_eq!(unresolved_count(&paths), 1);

    let blocked = apply_remote_upload_with_api(
        &paths,
        &default_target(),
        &mut api,
        &input(JPEG, false),
        Some("any-plan"),
        Some(("bot", "pass")),
    )
    .expect_err("an unresolved receipt must block another write");
    assert!(blocked.to_string().contains("unresolved remote upload"));
    assert_eq!(api.upload.requests.len(), 1, "the upload is never replayed");

    let report = reconcile_remote_mutation_with_api(
        &paths,
        &default_target(),
        &mut api,
        RemoteMutationOperation::Upload,
        mutation_id,
    )
    .expect("reconcile");
    assert_eq!(report.status, RemoteMutationReconciliationStatus::Applied);
    assert_eq!(api.upload.requests.len(), 1);
    let receipt = show_remote_mutation(
        &paths,
        &default_target(),
        RemoteMutationOperation::Upload,
        mutation_id,
    )
    .expect("receipt");
    assert_eq!(receipt.phase, "resolved");
    assert_eq!(receipt.terminal_outcome.as_deref(), Some("uploaded"));
}

#[test]
fn reconcile_distinguishes_not_applied_superseded_and_unprovable_uploads() {
    let ambiguous = |api: &mut MockApi, paths: &SyncProjectPaths, stored: bool| {
        if stored {
            api.upload.error_after_store = Some("timeout".to_string());
        } else {
            api.upload.error_before_store = Some("timeout".to_string());
        }
        let error = plan_and_apply(paths, api, &input(JPEG, true)).expect_err("ambiguous");
        match upload_error(&error) {
            RemoteUploadError::OutcomeAmbiguous { mutation_id, .. } => *mutation_id,
            other => panic!("expected an ambiguous outcome, got {other}"),
        }
    };
    let reconcile = |api: &mut MockApi, paths: &SyncProjectPaths, mutation_id| {
        reconcile_remote_mutation_with_api(
            paths,
            &default_target(),
            api,
            RemoteMutationOperation::Upload,
            mutation_id,
        )
        .expect("reconcile")
        .status
    };

    // The request never reached the server: the planned prior version is intact.
    let (_temp, paths, mut api) = established_project();
    api.upload.store(TITLE, OTHER_JPEG);
    let mutation_id = ambiguous(&mut api, &paths, false);
    assert_eq!(
        reconcile(&mut api, &paths, mutation_id),
        RemoteMutationReconciliationStatus::NotApplied
    );
    assert_eq!(unresolved_count(&paths), 0);

    // The upload applied and someone uploaded a newer version afterwards.
    let (_temp, paths, mut api) = established_project();
    let mutation_id = ambiguous(&mut api, &paths, true);
    api.upload.store(TITLE, THIRD_JPEG);
    assert_eq!(
        reconcile(&mut api, &paths, mutation_id),
        RemoteMutationReconciliationStatus::AppliedThenChanged
    );
    assert_eq!(unresolved_count(&paths), 0);

    // The remote changed to unrelated content and never showed our SHA-1.
    let (_temp, paths, mut api) = established_project();
    let mutation_id = ambiguous(&mut api, &paths, false);
    api.upload.store(TITLE, THIRD_JPEG);
    assert_eq!(
        reconcile(&mut api, &paths, mutation_id),
        RemoteMutationReconciliationStatus::ReconciliationRequired
    );
    assert_eq!(unresolved_count(&paths), 1);
    assert_eq!(api.upload.requests.len(), 1);
}

#[test]
fn plan_refuses_mislabelled_content_and_new_files_without_description_text() {
    let mut api = MockApi::default();
    let target = default_target();

    let mut mislabelled = input(JPEG, false);
    mislabelled.filename = "Sample photo.png";
    let error = plan_remote_upload_with_api(&target, &mut api, &mislabelled)
        .expect_err("jpeg bytes under a png name");
    assert!(error.to_string().contains("does not look like png"));

    let mut prefixed = input(JPEG, false);
    prefixed.filename = "File:Sample photo.jpg";
    assert!(plan_remote_upload_with_api(&target, &mut api, &prefixed).is_err());

    let mut undescribed = input(JPEG, false);
    undescribed.text = None;
    let error = plan_remote_upload_with_api(&target, &mut api, &undescribed)
        .expect_err("new file without description text");
    assert!(error.to_string().contains("--text-file"));
    assert_eq!(
        api.request_count, 2,
        "local validation failures must not reach the wiki"
    );
}
