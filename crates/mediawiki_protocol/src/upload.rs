use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::{MediaWikiClient, WikiWriteApi};

/// Uploads carry whole files, so they get a longer floor than the interactive
/// request timeout. A timed-out upload is ambiguous and is never replayed.
const UPLOAD_MIN_TIMEOUT_MS: u64 = 300_000;

/// One stored version of a local-repository file. `sha1` is absent only when
/// MediaWiki hides the version from the current reader.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RemoteFileVersion {
    pub sha1: Option<String>,
    pub size: u64,
    pub timestamp: String,
}

/// The read-only state of one `File:` title. `versions` is newest first and
/// empty when no local file exists under the title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFileObservation {
    pub title: String,
    pub versions: Vec<RemoteFileVersion>,
    /// False when MediaWiki reported more history than one response carried.
    pub history_complete: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct UploadRequest<'a> {
    /// Target name without the `File:` namespace prefix.
    pub filename: &'a str,
    pub bytes: &'a [u8],
    /// Initial description-page wikitext. MediaWiki uses it only when the
    /// upload creates the file page.
    pub text: Option<&'a str>,
    pub comment: &'a str,
    pub ignore_warnings: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UploadWarning {
    pub key: String,
    pub value: Value,
}

/// A decoded `action=upload` response. Only `Success` stores the file;
/// `Warnings` and `Rejected` are server answers that stored nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum UploadOutcome {
    Success {
        filename: String,
        sha1: Option<String>,
        timestamp: Option<String>,
    },
    Warnings(Vec<UploadWarning>),
    Rejected {
        code: String,
        info: String,
    },
}

pub trait WikiUploadApi: WikiWriteApi {
    /// Observe one exact `File:` title, including its stored version history.
    fn observe_file(&mut self, title: &str) -> Result<RemoteFileObservation>;
    /// List the `File:` titles whose current version has this SHA-1.
    fn find_files_by_sha1(&mut self, sha1: &str) -> Result<Vec<String>>;
    /// Send exactly one `action=upload` request. An `Err` means the outcome is
    /// unknown; implementations must never retry.
    fn upload_file(&mut self, request: &UploadRequest<'_>) -> Result<UploadOutcome>;
}

#[derive(Debug, Deserialize, Default)]
struct FileQueryResponse {
    #[serde(default)]
    query: FileQueryPayload,
    #[serde(default, rename = "continue")]
    continuation: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Deserialize, Default)]
struct FileQueryPayload {
    #[serde(default)]
    pages: Vec<FilePageItem>,
    #[serde(default)]
    allimages: Vec<AllImagesItem>,
}

#[derive(Debug, Deserialize)]
struct FilePageItem {
    title: String,
    invalid: Option<bool>,
    invalidreason: Option<String>,
    imagerepository: Option<String>,
    #[serde(default)]
    imageinfo: Vec<ImageInfoItem>,
}

#[derive(Debug, Deserialize)]
struct ImageInfoItem {
    timestamp: Option<String>,
    size: Option<u64>,
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AllImagesItem {
    title: String,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload: Option<UploadPayload>,
    error: Option<UploadErrorPayload>,
}

#[derive(Debug, Deserialize)]
struct UploadErrorPayload {
    code: Option<String>,
    info: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadPayload {
    result: Option<String>,
    filename: Option<String>,
    #[serde(default)]
    warnings: BTreeMap<String, Value>,
    imageinfo: Option<ImageInfoItem>,
}

impl WikiUploadApi for MediaWikiClient {
    fn observe_file(&mut self, title: &str) -> Result<RemoteFileObservation> {
        if title.trim().is_empty() {
            bail!("file observation requires a non-empty title");
        }
        let response = self.query_json(&[
            ("action", "query".to_string()),
            ("titles", title.to_string()),
            ("prop", "imageinfo".to_string()),
            ("iiprop", "sha1|size|timestamp".to_string()),
            ("iilimit", "max".to_string()),
        ])?;
        decode_file_observation(response, title)
    }

    fn find_files_by_sha1(&mut self, sha1: &str) -> Result<Vec<String>> {
        if sha1.len() != 40 || !sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("SHA-1 lookup requires 40 hexadecimal characters");
        }
        let mut titles = Vec::new();
        let mut continuation = None::<String>;
        loop {
            let mut params = vec![
                ("action", "query".to_string()),
                ("list", "allimages".to_string()),
                ("aisha1", sha1.to_ascii_lowercase()),
                ("aiprop", "sha1".to_string()),
                ("ailimit", "max".to_string()),
            ];
            if let Some(token) = &continuation {
                params.push(("aicontinue", token.clone()));
            }
            let parsed: FileQueryResponse = serde_json::from_value(self.query_json(&params)?)
                .context("failed to decode allimages response")?;
            titles.extend(parsed.query.allimages.into_iter().map(|item| item.title));
            continuation = parsed
                .continuation
                .and_then(|values| values.get("aicontinue").cloned())
                .and_then(|value| value.as_str().map(ToString::to_string));
            if continuation.is_none() {
                titles.sort();
                titles.dedup();
                return Ok(titles);
            }
        }
    }

    fn upload_file(&mut self, request: &UploadRequest<'_>) -> Result<UploadOutcome> {
        let token = self.ensure_csrf_token()?;
        let mut form = Form::new();
        for (key, value) in upload_text_fields(request, &token) {
            form = form.text(key, value);
        }
        let part = Part::bytes(request.bytes.to_vec())
            .file_name(request.filename.to_string())
            .mime_str("application/octet-stream")
            .context("failed to build upload file part")?;
        form = form.part("file", part);

        // A timeout or 5xx after a mutation leaves the outcome ambiguous, so
        // this request is sent exactly once.
        self.apply_rate_limit(true);
        let response = self
            .client
            .post(&self.config.api_url)
            .header("User-Agent", self.config.user_agent.clone())
            .timeout(Duration::from_millis(
                self.config.timeout_ms.max(UPLOAD_MIN_TIMEOUT_MS),
            ))
            .multipart(form)
            .send()
            .context("failed to call MediaWiki API")?;
        let status = response.status();
        if !status.is_success() {
            bail!("MediaWiki API request failed with HTTP {status}");
        }
        let payload: Value = response
            .json()
            .context("failed to decode MediaWiki API JSON response")?;
        decode_upload_response(payload)
    }
}

fn upload_text_fields(request: &UploadRequest<'_>, token: &str) -> Vec<(&'static str, String)> {
    let mut fields = vec![
        ("action", "upload".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
        ("filename", request.filename.to_string()),
        ("comment", request.comment.to_string()),
    ];
    if let Some(text) = request.text {
        fields.push(("text", text.to_string()));
    }
    if request.ignore_warnings {
        fields.push(("ignorewarnings", "1".to_string()));
    }
    fields.push(("token", token.to_string()));
    fields
}

fn decode_file_observation(
    response: Value,
    requested_title: &str,
) -> Result<RemoteFileObservation> {
    let parsed: FileQueryResponse =
        serde_json::from_value(response).context("failed to decode imageinfo response")?;
    let history_complete = parsed.continuation.is_none();
    let mut pages = parsed.query.pages;
    if pages.len() != 1 {
        bail!(
            "MediaWiki returned {} pages for single-file observation {requested_title:?}",
            pages.len()
        );
    }
    let page = pages.remove(0);
    if page.invalid.unwrap_or(false) {
        bail!(
            "MediaWiki rejected file title {requested_title:?}: {}",
            page.invalidreason.as_deref().unwrap_or("invalid title")
        );
    }
    let repository = page.imagerepository.unwrap_or_default();
    if !repository.is_empty() && repository != "local" {
        bail!(
            "{} is provided by the {repository:?} file repository, not by this wiki; only local files can be observed",
            page.title
        );
    }
    let versions = page
        .imageinfo
        .into_iter()
        .map(|item| {
            Ok(RemoteFileVersion {
                sha1: item
                    .sha1
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.to_ascii_lowercase()),
                size: item.size.unwrap_or(0),
                timestamp: item
                    .timestamp
                    .filter(|value| !value.trim().is_empty())
                    .with_context(|| {
                        format!("imageinfo for {} omitted a version timestamp", page.title)
                    })?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(RemoteFileObservation {
        title: page.title,
        versions,
        history_complete,
    })
}

fn decode_upload_response(response: Value) -> Result<UploadOutcome> {
    let parsed: UploadResponse =
        serde_json::from_value(response).context("failed to decode upload response")?;
    if let Some(error) = parsed.error {
        return Ok(UploadOutcome::Rejected {
            code: error.code.unwrap_or_else(|| "unknown_error".to_string()),
            info: error.info.unwrap_or_else(|| "unknown info".to_string()),
        });
    }
    let upload = parsed
        .upload
        .ok_or_else(|| anyhow::anyhow!("missing upload payload in API response"))?;
    match upload.result.as_deref() {
        Some("Success") => Ok(UploadOutcome::Success {
            filename: upload
                .filename
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("upload response omitted the stored filename"))?,
            sha1: upload
                .imageinfo
                .as_ref()
                .and_then(|info| info.sha1.as_ref())
                .map(|value| value.to_ascii_lowercase()),
            timestamp: upload.imageinfo.and_then(|info| info.timestamp),
        }),
        Some("Warning") => {
            if upload.warnings.is_empty() {
                bail!("upload response reported a warning result without warning keys");
            }
            Ok(UploadOutcome::Warnings(
                upload
                    .warnings
                    .into_iter()
                    .map(|(key, value)| UploadWarning { key, value })
                    .collect(),
            ))
        }
        other => bail!(
            "upload response returned unsupported result {:?}",
            other.unwrap_or("<missing>")
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::thread;

    use serde_json::json;
    use tiny_http::{Header, Method, Response, Server};

    use super::*;
    use crate::client::MediaWikiClientConfig;

    struct ReceivedUpload {
        content_type: String,
        body: Vec<u8>,
    }

    /// Serve one CSRF token query and then answer every POST with `status` and
    /// `response`, returning the POST requests that actually arrived.
    fn serve_uploads(
        status: u16,
        response: Value,
        expected_posts: usize,
    ) -> (String, thread::JoinHandle<Vec<ReceivedUpload>>) {
        let server = Server::http("127.0.0.1:0").expect("bind fixture server");
        let api_url = format!("http://{}/api.php", server.server_addr());
        let handle = thread::spawn(move || {
            let json_header =
                Header::from_bytes("Content-Type", "application/json").expect("header");
            let mut received = Vec::new();
            while received.len() < expected_posts {
                let Ok(mut request) = server.recv() else {
                    break;
                };
                if *request.method() == Method::Post {
                    let content_type = request
                        .headers()
                        .iter()
                        .find(|header| header.field.equiv("Content-Type"))
                        .map(|header| header.value.as_str().to_string())
                        .unwrap_or_default();
                    let mut body = Vec::new();
                    request.as_reader().read_to_end(&mut body).expect("body");
                    received.push(ReceivedUpload { content_type, body });
                    let _ = request.respond(
                        Response::from_string(response.to_string())
                            .with_status_code(status)
                            .with_header(json_header.clone()),
                    );
                } else {
                    let tokens = json!({"query": {"tokens": {"csrftoken": "csrf+\\"}}});
                    let _ = request.respond(
                        Response::from_string(tokens.to_string()).with_header(json_header.clone()),
                    );
                }
            }
            received
        });
        (api_url, handle)
    }

    fn client(api_url: String) -> MediaWikiClient {
        MediaWikiClient::new(MediaWikiClientConfig {
            api_url,
            user_agent: "wikitool-upload-test".to_string(),
            timeout_ms: 10_000,
            rate_limit_read_ms: 0,
            rate_limit_write_ms: 0,
            max_retries: 2,
            retry_delay_ms: 1,
            mark_edits_as_bot: false,
        })
        .expect("client")
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Split a multipart/form-data body into its named parts.
    fn multipart_parts(upload: &ReceivedUpload) -> BTreeMap<String, (String, Vec<u8>)> {
        let (media_type, boundary) = upload
            .content_type
            .split_once("boundary=")
            .expect("multipart boundary parameter");
        assert!(media_type.trim_start().starts_with("multipart/form-data"));
        let delimiter = format!("--{}", boundary.trim_matches('"')).into_bytes();
        let mut parts = BTreeMap::new();
        let mut rest = upload.body.as_slice();
        while let Some(start) = find(rest, &delimiter) {
            rest = &rest[start + delimiter.len()..];
            if rest.starts_with(b"--") {
                break;
            }
            let end = find(rest, &delimiter).expect("closing multipart delimiter");
            let part = &rest[2..end - 2];
            let split = find(part, b"\r\n\r\n").expect("multipart part headers");
            let headers = String::from_utf8(part[..split].to_vec()).expect("part headers");
            let (_, name) = headers.split_once("name=\"").expect("part name");
            let (name, _) = name.split_once('"').expect("part name end");
            parts.insert(name.to_string(), (headers, part[split + 4..].to_vec()));
        }
        parts
    }

    #[test]
    fn upload_sends_one_multipart_request_with_exact_bytes_and_fields() {
        let (api_url, server) = serve_uploads(
            200,
            json!({"upload": {
                "result": "Success",
                "filename": "Sample_photo.jpg",
                "imageinfo": {"timestamp": "2026-11-10T00:00:00Z", "size": 6, "sha1": "ab"}
            }}),
            1,
        );
        let bytes = [0xFF, 0xD8, 0xFF, 0x00, b'\r', b'\n'];
        let outcome = client(api_url)
            .upload_file(&UploadRequest {
                filename: "Sample photo.jpg",
                bytes: &bytes,
                text: Some("== Summary ==\nline two"),
                comment: "upload comment",
                ignore_warnings: false,
            })
            .expect("upload");
        assert_eq!(
            outcome,
            UploadOutcome::Success {
                filename: "Sample_photo.jpg".to_string(),
                sha1: Some("ab".to_string()),
                timestamp: Some("2026-11-10T00:00:00Z".to_string()),
            }
        );

        let received = server.join().expect("fixture server");
        assert_eq!(received.len(), 1);
        let parts = multipart_parts(&received[0]);
        let text = |name: &str| String::from_utf8(parts[name].1.clone()).expect("text field");
        assert_eq!(text("action"), "upload");
        assert_eq!(text("format"), "json");
        assert_eq!(text("filename"), "Sample photo.jpg");
        assert_eq!(text("comment"), "upload comment");
        assert_eq!(text("text"), "== Summary ==\nline two");
        assert_eq!(text("token"), "csrf+\\");
        assert!(!parts.contains_key("ignorewarnings"));
        assert_eq!(parts["file"].1, bytes);
        assert!(parts["file"].0.contains("filename=\"Sample photo.jpg\""));
    }

    #[test]
    fn failed_upload_response_is_an_error_and_is_never_replayed() {
        let (api_url, server) = serve_uploads(503, json!({"unavailable": true}), 1);
        let mut client = client(api_url);
        let error = client
            .upload_file(&request(None, true))
            .expect_err("a 503 leaves the outcome unknown");
        assert!(error.to_string().contains("503"));

        let received = server.join().expect("fixture server");
        assert_eq!(received.len(), 1, "the upload must be sent exactly once");
        assert_eq!(
            String::from_utf8(multipart_parts(&received[0])["ignorewarnings"].1.clone())
                .expect("flag"),
            "1"
        );
    }

    fn request(text: Option<&'static str>, ignore_warnings: bool) -> UploadRequest<'static> {
        UploadRequest {
            filename: "Sample.jpg",
            bytes: b"bytes",
            text,
            comment: "upload comment",
            ignore_warnings,
        }
    }

    #[test]
    fn upload_fields_send_ignorewarnings_and_text_only_when_requested() {
        let plain = upload_text_fields(&request(None, false), "token+\\");
        assert!(!plain.iter().any(|(key, _)| *key == "ignorewarnings"));
        assert!(!plain.iter().any(|(key, _)| *key == "text"));
        assert_eq!(plain.last(), Some(&("token", "token+\\".to_string())));

        let replacing = upload_text_fields(&request(Some("description"), true), "token+\\");
        assert!(
            replacing
                .iter()
                .any(|(key, value)| *key == "ignorewarnings" && value == "1")
        );
        assert!(
            replacing
                .iter()
                .any(|(key, value)| *key == "text" && value == "description")
        );
    }

    #[test]
    fn decodes_success_warning_and_error_upload_responses() {
        let success = decode_upload_response(json!({
            "upload": {
                "result": "Success",
                "filename": "Sample.jpg",
                "imageinfo": {
                    "timestamp": "2026-11-10T00:00:00Z",
                    "size": 5,
                    "sha1": "ABCDEF0123456789ABCDEF0123456789ABCDEF01"
                }
            }
        }))
        .expect("success response");
        assert_eq!(
            success,
            UploadOutcome::Success {
                filename: "Sample.jpg".to_string(),
                sha1: Some("abcdef0123456789abcdef0123456789abcdef01".to_string()),
                timestamp: Some("2026-11-10T00:00:00Z".to_string()),
            }
        );

        let warning = decode_upload_response(json!({
            "upload": {
                "result": "Warning",
                "warnings": {"exists": "Sample.jpg", "duplicate": ["Other.jpg"]},
                "filekey": "stash.jpg"
            }
        }))
        .expect("warning response");
        assert_eq!(
            warning,
            UploadOutcome::Warnings(vec![
                UploadWarning {
                    key: "duplicate".to_string(),
                    value: json!(["Other.jpg"]),
                },
                UploadWarning {
                    key: "exists".to_string(),
                    value: json!("Sample.jpg"),
                },
            ])
        );

        let rejected = decode_upload_response(json!({
            "error": {"code": "permissiondenied", "info": "not allowed"}
        }))
        .expect("error response");
        assert_eq!(
            rejected,
            UploadOutcome::Rejected {
                code: "permissiondenied".to_string(),
                info: "not allowed".to_string(),
            }
        );
    }

    #[test]
    fn rejects_upload_responses_that_prove_no_outcome() {
        assert!(decode_upload_response(json!({"batchcomplete": true})).is_err());
        assert!(decode_upload_response(json!({"upload": {"result": "Continue"}})).is_err());
        assert!(decode_upload_response(json!({"upload": {"result": "Warning"}})).is_err());
        assert!(decode_upload_response(json!({"upload": {"result": "Success"}})).is_err());
    }

    #[test]
    fn file_observation_distinguishes_missing_present_and_foreign_files() {
        let missing = decode_file_observation(
            json!({"query": {"pages": [{
                "ns": 6, "title": "File:Sample.jpg", "missing": true, "imagerepository": ""
            }]}}),
            "File:Sample.jpg",
        )
        .expect("missing file");
        assert!(missing.versions.is_empty());
        assert!(missing.history_complete);

        let present = decode_file_observation(
            json!({
                "continue": {"iistart": "2025-01-01T00:00:00Z", "continue": "||"},
                "query": {"pages": [{
                    "pageid": 9, "ns": 6, "title": "File:Sample.jpg",
                    "imagerepository": "local",
                    "imageinfo": [
                        {"timestamp": "2026-01-02T00:00:00Z", "size": 7, "sha1": "AA"},
                        {"timestamp": "2026-01-01T00:00:00Z", "size": 5, "filehidden": true}
                    ]
                }]}
            }),
            "File:Sample.jpg",
        )
        .expect("present file");
        assert_eq!(present.title, "File:Sample.jpg");
        assert_eq!(present.versions[0].sha1.as_deref(), Some("aa"));
        assert_eq!(present.versions[0].size, 7);
        assert_eq!(present.versions[1].sha1, None);
        assert!(!present.history_complete);

        let foreign = decode_file_observation(
            json!({"query": {"pages": [{
                "ns": 6, "title": "File:Sample.jpg", "missing": true, "known": true,
                "imagerepository": "shared",
                "imageinfo": [{"timestamp": "2026-01-02T00:00:00Z", "size": 7, "sha1": "aa"}]
            }]}}),
            "File:Sample.jpg",
        )
        .expect_err("foreign repository file must not be treated as local");
        assert!(foreign.to_string().contains("shared"));
    }
}
