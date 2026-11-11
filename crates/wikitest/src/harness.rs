//! Plain helpers for Wikitool CLI tests written as ordinary `cargo test` integration tests.
//!
//! A test creates a [`Project`] (an isolated directory that is removed when the test passes and
//! kept when it panics), runs the `wikitool` binary in it, and checks the returned [`Output`].
//! [`FakeWiki`] serves a MediaWiki fixture on loopback and records every request it receives.
//! Helpers panic instead of returning errors, reporting the test's own line and including the
//! command, project path and captured output, so a failing assertion explains itself.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::Value;

use crate::artifact::{atomic_write, resolve_output_path};
use crate::mediawiki::{
    MediaWikiDeleteLog, MediaWikiFixture, MediaWikiObservation, MediaWikiRequest, MediaWikiService,
};
use crate::model::MediaWikiPage;
use crate::process::run_bounded_with_stdin;

/// Default per-command limit; the whole process tree is killed when it is exceeded.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
/// Bytes kept per stream, as in the JSON runner. Longer output fails the command.
const OUTPUT_BUDGET: usize = 2 * 1024 * 1024;
/// Bytes of each stream shown in a failure message.
const DISPLAY_LIMIT: usize = 16 * 1024;

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(0);

/// Where a failed test's project keeps the fake wiki's observation.
const FAKE_WIKI_OBSERVATION: &str = ".wikitest-private/fake-wiki-observation.json";

/// An isolated Wikitool project directory bound to one `wikitool` executable.
///
/// A project made with [`Project::with_wiki`] borrows its [`FakeWiki`], so that a failed test
/// can keep the wiki's observation beside the project.
pub struct Project<'w> {
    root: PathBuf,
    wikitool: PathBuf,
    environment: BTreeMap<String, String>,
    timeout: Cell<Duration>,
    commands_run: Cell<usize>,
    wiki: Option<&'w FakeWiki>,
}

/// The result of one `wikitool` command.
#[derive(Debug)]
pub struct Output {
    /// The command as written by the test, without the runner-owned path arguments.
    pub command: String,
    /// `None` when the process ended without an exit code.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    project: PathBuf,
}

/// The local MediaWiki fixture server. It stops when dropped.
pub struct FakeWiki {
    service: MediaWikiService,
}

impl<'w> Project<'w> {
    /// Creates an empty project with an empty `.env`, so no developer `.env` applies.
    #[track_caller]
    pub fn new(wikitool: impl AsRef<Path>) -> Self {
        let wikitool = wikitool.as_ref();
        let wikitool = match fs::canonicalize(wikitool) {
            Ok(path) => path,
            Err(error) => panic!(
                "cannot resolve wikitool executable {}: {error}",
                wikitool.display()
            ),
        };
        let root = create_project_directory();
        if let Err(error) = atomic_write(&root.join(".env"), b"") {
            panic!("cannot write {}/.env: {error:#}", shown(&root));
        }
        Self {
            root,
            wikitool,
            environment: BTreeMap::new(),
            timeout: Cell::new(DEFAULT_TIMEOUT),
            commands_run: Cell::new(0),
            wiki: None,
        }
    }

    /// Creates a project whose documentation and extension-discovery lookups are pinned to
    /// `wiki`, with retries disabled, exactly as the JSON runner does for fixture scenarios.
    /// If the test fails, the wiki's observation is written into the kept project.
    #[track_caller]
    pub fn with_wiki(wikitool: impl AsRef<Path>, wiki: &'w FakeWiki) -> Self {
        let mut project = Self::new(wikitool);
        project.wiki = Some(wiki);
        for key in [
            "WIKITOOL_DOCS_API_URL",
            "WIKITOOL_INSTALLED_EXTENSIONS_API_URL",
        ] {
            project
                .environment
                .insert(key.to_owned(), wiki.api_url().to_owned());
        }
        project
            .environment
            .insert("WIKITOOL_DOCS_RETRIES".to_owned(), "0".to_owned());
        project
            .environment
            .insert("WIKITOOL_DOCS_TIMEOUT_MS".to_owned(), "3000".to_owned());
        project
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Changes the limit for later commands.
    pub fn set_timeout(&self, timeout: Duration) {
        self.timeout.set(timeout);
    }

    /// Runs `wikitool <args>` in the project.
    #[track_caller]
    pub fn run(&self, args: &[&str]) -> Output {
        self.run_with_env(args, &[])
    }

    /// Runs `wikitool <args>` with extra environment variables.
    ///
    /// The project root, data directory and config are always passed explicitly, and every
    /// inherited `WIKITOOL_*` variable is removed before `env` is applied. The command panics
    /// if it times out or its output exceeds the capture budget.
    #[track_caller]
    pub fn run_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.run_command(args, env, None)
    }

    /// Runs `wikitool <args>` with `stdin` as its standard input, which is closed after the
    /// bytes are written. Paths and environment are controlled exactly as in [`Self::run`];
    /// other commands get an empty standard input.
    #[track_caller]
    pub fn run_with_stdin(&self, args: &[&str], stdin: &[u8]) -> Output {
        self.run_command(args, &[], Some(stdin))
    }

    #[track_caller]
    fn run_command(&self, args: &[&str], env: &[(&str, &str)], stdin: Option<&[u8]>) -> Output {
        let index = self.commands_run.get();
        self.commands_run.set(index + 1);
        let mut argv = vec![
            "--project-root".to_owned(),
            self.root.to_string_lossy().into_owned(),
            "--data-dir".to_owned(),
            self.root
                .join(".wikitool/data")
                .to_string_lossy()
                .into_owned(),
            "--config".to_owned(),
            self.root
                .join(".wikitool/config.toml")
                .to_string_lossy()
                .into_owned(),
        ];
        argv.extend(args.iter().map(|arg| (*arg).to_owned()));
        let mut environment = env
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>();
        // Fixture endpoints win over anything the test passes, as in the JSON runner.
        environment.extend(self.environment.clone());
        let command = format!("wikitool {}", args.join(" "));
        let output_directory = self.root.join(".wikitest-private/process-output");
        let outcome = match run_bounded_with_stdin(
            &self.wikitool,
            &argv,
            &self.root,
            &environment,
            stdin,
            self.timeout.get(),
            OUTPUT_BUDGET,
            &output_directory.join(format!("{index:03}.stdout.txt")),
            &output_directory.join(format!("{index:03}.stderr.txt")),
        ) {
            Ok(outcome) => outcome,
            Err(error) => panic!(
                "could not run `{command}` in {}: {error:#}",
                shown(&self.root)
            ),
        };
        let output = Output {
            command,
            code: outcome.status.code(),
            stdout: String::from_utf8_lossy(&outcome.stdout.bytes).into_owned(),
            stderr: String::from_utf8_lossy(&outcome.stderr.bytes).into_owned(),
            project: self.root.clone(),
        };
        if outcome.timed_out {
            panic!(
                "command exceeded {} ms and its process tree was killed\n{output}",
                self.timeout.get().as_millis()
            );
        }
        if outcome.stdout.truncated || outcome.stderr.truncated {
            panic!("command printed more than {OUTPUT_BUDGET} bytes on one stream\n{output}");
        }
        output
    }

    /// Copies `source` to the project-relative path `target`, creating parent directories.
    #[track_caller]
    pub fn install(&self, source: impl AsRef<Path>, target: &str) {
        let source = source.as_ref();
        let bytes = match fs::read(source) {
            Ok(bytes) => bytes,
            Err(error) => panic!("cannot read fixture {}: {error}", source.display()),
        };
        let destination = self.resolve(target);
        if let Err(error) = atomic_write(&destination, &bytes) {
            panic!("cannot install {}: {error:#}", shown(&destination));
        }
    }

    /// Reads a project-relative text file.
    #[track_caller]
    pub fn read(&self, relative: &str) -> String {
        let path = self.resolve(relative);
        match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => panic!("cannot read {}: {error}", shown(&path)),
        }
    }

    /// Panics, showing the file, unless the project-relative file contains `needle`.
    #[track_caller]
    pub fn assert_file_contains(&self, relative: &str, needle: &str) {
        let text = self.read(relative);
        if !text.contains(needle) {
            panic!(
                "{relative} does not contain {needle:?}\nproject: {}\n--- {relative} ---\n{}",
                shown(&self.root),
                shorten(&text)
            );
        }
    }

    /// Panics, showing the file, if the project-relative file contains `needle`.
    #[track_caller]
    pub fn assert_file_lacks(&self, relative: &str, needle: &str) {
        let text = self.read(relative);
        if text.contains(needle) {
            panic!(
                "{relative} contains {needle:?}\nproject: {}\n--- {relative} ---\n{}",
                shown(&self.root),
                shorten(&text)
            );
        }
    }

    /// Panics unless the project-relative path is a plain file (not a directory or symlink).
    #[track_caller]
    pub fn assert_file_exists(&self, relative: &str) {
        let path = self.resolve(relative);
        let plain = fs::symlink_metadata(&path)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink());
        if !plain {
            panic!(
                "{relative} is not a plain file\nproject: {}",
                shown(&self.root)
            );
        }
    }

    /// Panics unless nothing exists at the project-relative path.
    #[track_caller]
    pub fn assert_missing(&self, relative: &str) {
        let path = self.resolve(relative);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => panic!("{relative} exists\nproject: {}", shown(&self.root)),
            Err(error) => panic!("cannot inspect {}: {error}", shown(&path)),
        }
    }

    #[track_caller]
    fn resolve(&self, relative: &str) -> PathBuf {
        match resolve_output_path(&self.root, relative) {
            Ok(path) => path,
            Err(error) => panic!("invalid project path {relative:?}: {error:#}"),
        }
    }
}

impl Drop for Project<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let mut message = format!(
                "wikitest: kept project of failed test: {}",
                shown(&self.root)
            );
            if let Some(wiki) = self.wiki {
                let path = self.root.join(FAKE_WIKI_OBSERVATION);
                match wiki.write_observation(&path) {
                    Ok(()) => message.push_str(&format!(
                        "\nwikitest: fake wiki observation (tokens and passwords redacted): {}",
                        shown(&path)
                    )),
                    Err(error) => message.push_str(&format!(
                        "\nwikitest: could not write the fake wiki observation: {error:#}"
                    )),
                }
            }
            eprintln!("{message}");
        } else if let Err(error) = fs::remove_dir_all(&self.root) {
            eprintln!(
                "wikitest: could not remove project {}: {error}",
                shown(&self.root)
            );
        }
    }
}

fn create_project_directory() -> PathBuf {
    let base = std::env::temp_dir().join("wikitest-projects");
    if let Err(error) = fs::create_dir_all(&base) {
        panic!("cannot create {}: {error}", base.display());
    }
    // Name the directory after the test so a kept project is easy to find.
    let thread = std::thread::current();
    let label = thread
        .name()
        .and_then(|name| name.rsplit("::").next())
        .unwrap_or("project")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(48)
        .collect::<String>();
    loop {
        let candidate = base.join(format!(
            "{label}-{}-{}",
            std::process::id(),
            NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => {
                // The JSON runner also passes the canonical path to Wikitool.
                return match fs::canonicalize(&candidate) {
                    Ok(path) => path,
                    Err(error) => panic!("cannot resolve {}: {error}", candidate.display()),
                };
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("cannot create {}: {error}", candidate.display()),
        }
    }
}

impl Output {
    /// Panics unless the command exited with `expected`.
    #[track_caller]
    pub fn assert_code(&self, expected: i32) -> &Self {
        if self.code != Some(expected) {
            panic!("expected exit code {expected}\n{self}");
        }
        self
    }

    /// Parses stdout as one JSON document.
    #[track_caller]
    pub fn json(&self) -> Value {
        match serde_json::from_str(&self.stdout) {
            Ok(value) => value,
            Err(error) => panic!("stdout is not JSON: {error}\n{self}"),
        }
    }

    /// The JSON value at `pointer` in stdout. Panics if it is absent.
    #[track_caller]
    pub fn json_at(&self, pointer: &str) -> Value {
        match self.json().pointer(pointer) {
            Some(value) => value.clone(),
            None => panic!("stdout has no JSON value at {pointer}\n{self}"),
        }
    }

    /// The nonblank string at `pointer`, such as a plan ID to pass to a later command.
    #[track_caller]
    pub fn json_str(&self, pointer: &str) -> String {
        match self.json_at(pointer) {
            Value::String(value) if !value.trim().is_empty() => value,
            other => panic!("expected a nonblank string at {pointer}, found {other}\n{self}"),
        }
    }

    /// The unsigned integer at `pointer`.
    #[track_caller]
    pub fn json_u64(&self, pointer: &str) -> u64 {
        let value = self.json_at(pointer);
        match value.as_u64() {
            Some(number) => number,
            None => panic!("expected an unsigned integer at {pointer}, found {value}\n{self}"),
        }
    }

    /// The array at `pointer`.
    #[track_caller]
    pub fn json_array(&self, pointer: &str) -> Vec<Value> {
        match self.json_at(pointer) {
            Value::Array(items) => items,
            other => panic!("expected an array at {pointer}, found {other}\n{self}"),
        }
    }

    /// Panics unless the JSON value at `pointer` equals `expected`.
    #[track_caller]
    pub fn assert_json(&self, pointer: &str, expected: impl Into<Value>) -> &Self {
        let expected = expected.into();
        let observed = self.json().pointer(pointer).cloned();
        if observed.as_ref() != Some(&expected) {
            let observed = match observed {
                Some(value) => value.to_string(),
                None => "<missing>".to_owned(),
            };
            panic!("at {pointer}: expected {expected}, observed {observed}\n{self}");
        }
        self
    }

    /// Panics unless the array at `pointer` has `member` as one of its items.
    #[track_caller]
    pub fn assert_json_contains(&self, pointer: &str, member: impl Into<Value>) -> &Self {
        let member = member.into();
        if !self.json_array(pointer).contains(&member) {
            panic!("the array at {pointer} has no item {member}\n{self}");
        }
        self
    }

    /// Panics unless some item of the array at `pointer` has `expected` at `item_pointer`.
    #[track_caller]
    pub fn assert_json_item(
        &self,
        pointer: &str,
        item_pointer: &str,
        expected: impl Into<Value>,
    ) -> &Self {
        let expected = expected.into();
        let items = self.json_array(pointer);
        if !items
            .iter()
            .any(|item| item.pointer(item_pointer) == Some(&expected))
        {
            panic!("no item of the array at {pointer} has {expected} at {item_pointer}\n{self}");
        }
        self
    }

    /// Panics unless the value at `pointer` is an unsigned integer of at least `minimum`.
    #[track_caller]
    pub fn assert_json_at_least(&self, pointer: &str, minimum: u64) -> &Self {
        let observed = self.json_u64(pointer);
        if observed < minimum {
            panic!("at {pointer}: expected at least {minimum}, observed {observed}\n{self}");
        }
        self
    }

    /// Panics unless stdout contains `needle`.
    #[track_caller]
    pub fn assert_stdout_contains(&self, needle: &str) -> &Self {
        if !self.stdout.contains(needle) {
            panic!("stdout lacks {needle:?}\n{self}");
        }
        self
    }

    /// Panics if stdout contains `needle`.
    #[track_caller]
    pub fn assert_stdout_lacks(&self, needle: &str) -> &Self {
        if self.stdout.contains(needle) {
            panic!("stdout contains {needle:?}\n{self}");
        }
        self
    }

    /// Panics unless stderr contains `needle`.
    #[track_caller]
    pub fn assert_stderr_contains(&self, needle: &str) -> &Self {
        if !self.stderr.contains(needle) {
            panic!("stderr lacks {needle:?}\n{self}");
        }
        self
    }
}

impl fmt::Display for Output {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "command: {}", self.command)?;
        writeln!(formatter, "project: {}", shown(&self.project))?;
        match self.code {
            Some(code) => writeln!(formatter, "exit code: {code}")?,
            None => writeln!(formatter, "exit code: <none>")?,
        }
        writeln!(formatter, "--- stdout ---\n{}", shorten(&self.stdout))?;
        write!(formatter, "--- stderr ---\n{}", shorten(&self.stderr))
    }
}

fn shorten(text: &str) -> &str {
    if text.len() <= DISPLAY_LIMIT {
        return text;
    }
    let mut end = DISPLAY_LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// A path for messages, without the Windows verbatim prefix that canonicalization adds.
fn shown(path: &Path) -> String {
    let text = path.display().to_string();
    match text.strip_prefix(r"\\?\") {
        Some(plain) if !plain.starts_with("UNC") => plain.to_owned(),
        _ => text,
    }
}

impl FakeWiki {
    /// Starts a server from a `wikitest.mediawiki-fixture.v5` JSON file.
    #[track_caller]
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        match MediaWikiFixture::from_path(path) {
            Ok(fixture) => Self::start(fixture),
            Err(error) => panic!("invalid MediaWiki fixture {}: {error:#}", path.display()),
        }
    }

    /// Starts a server from a fixture value. Its public fields configure pages, delete logs
    /// and the injected ambiguous or `missingtitle` failures.
    #[track_caller]
    pub fn start(fixture: MediaWikiFixture) -> Self {
        match MediaWikiService::start(fixture) {
            Ok(service) => Self { service },
            Err(error) => panic!("cannot start fake MediaWiki: {error:#}"),
        }
    }

    /// The `api.php` URL to configure in Wikitool.
    pub fn api_url(&self) -> &str {
        self.service.endpoint()
    }

    /// Creates or replaces a page, as a concurrent remote edit would.
    #[track_caller]
    pub fn set_page(&self, page: MediaWikiPage) {
        let title = page.title.clone();
        if let Err(error) = self.service.update_page(page) {
            panic!("cannot set fake page {title}: {error:#}");
        }
    }

    /// Current pages, delete logs and the ordered request log.
    #[track_caller]
    pub fn observe(&self) -> MediaWikiObservation {
        match self.service.observation() {
            Ok(observation) => observation,
            Err(error) => panic!("cannot observe fake MediaWiki: {error:#}"),
        }
    }

    #[track_caller]
    pub fn page(&self, title: &str) -> Option<MediaWikiPage> {
        self.observe()
            .pages
            .into_iter()
            .find(|page| page.title == title)
    }

    /// Delete-log entries for `title`, including those the fixture started with.
    #[track_caller]
    pub fn delete_logs(&self, title: &str) -> Vec<MediaWikiDeleteLog> {
        self.observe()
            .delete_logs
            .into_iter()
            .filter(|log| log.title == title)
            .collect()
    }

    /// Panics, listing every request received, unless exactly `expected` requests with
    /// `method` include every parameter pair in `params`.
    #[track_caller]
    pub fn assert_requests(&self, method: &str, params: &[(&str, &str)], expected: usize) {
        let requests = self.observe().requests;
        let observed = requests
            .iter()
            .filter(|request| request_matches(request, method, params))
            .count();
        if observed != expected {
            panic!(
                "expected {expected} {method} request(s) with {params:?}, observed {observed}\n\
                 requests received, in order:\n{}",
                render_requests(&requests)
            );
        }
    }

    /// Writes the observation as JSON, with secrets redacted as in request-failure messages.
    /// Used while a test is already panicking, so it reports errors instead of panicking.
    fn write_observation(&self, path: &Path) -> anyhow::Result<()> {
        let mut observation = self.service.observation()?;
        for request in &mut observation.requests {
            for (key, value) in &mut request.params {
                if is_secret(key) {
                    *value = "<redacted>".to_owned();
                }
            }
        }
        let mut bytes = serde_json::to_vec_pretty(&observation)?;
        bytes.push(b'\n');
        atomic_write(path, &bytes)
    }
}

fn request_matches(request: &MediaWikiRequest, method: &str, params: &[(&str, &str)]) -> bool {
    request.method == method
        && params
            .iter()
            .all(|(key, value)| request.params.get(*key).map(String::as_str) == Some(*value))
}

/// Request parameters that carry login or CSRF tokens or passwords.
fn is_secret(key: &str) -> bool {
    matches!(key, "lgpassword" | "lgtoken" | "token")
}

fn render_requests(requests: &[MediaWikiRequest]) -> String {
    let mut rendered = String::new();
    for (index, request) in requests.iter().enumerate() {
        rendered.push_str(&format!("{index:4} {}", request.method));
        for (key, value) in &request.params {
            let value = match key.as_str() {
                key if is_secret(key) => "<redacted>".to_owned(),
                _ if value.chars().count() > 60 => {
                    format!("{}...", value.chars().take(60).collect::<String>())
                }
                _ => value.clone(),
            };
            rendered.push_str(&format!(" {key}={value:?}"));
        }
        rendered.push('\n');
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    // A scenario test cannot observe its own cleanup, so this checks the two outcomes directly.
    #[test]
    fn project_is_removed_after_success_and_kept_after_a_panic() {
        let executable = std::env::current_exe().expect("test executable");

        let passed = Project::new(&executable);
        let passed_path = passed.path().to_path_buf();
        assert!(passed_path.join(".env").is_file());
        drop(passed);
        assert!(!passed_path.exists(), "passing project was not removed");

        let (sender, receiver) = std::sync::mpsc::channel();
        let failed = std::thread::spawn(move || {
            let project = Project::new(&executable);
            sender
                .send(project.path().to_path_buf())
                .expect("send path");
            panic!("simulated test failure");
        })
        .join();
        assert!(failed.is_err(), "simulated failure did not panic");
        let kept_path = receiver.recv().expect("kept path");
        let kept = kept_path.is_dir();
        let _ = fs::remove_dir_all(&kept_path);
        assert!(kept, "failed project was not kept");
    }

    #[test]
    fn failed_project_keeps_the_redacted_wiki_observation() {
        use std::io::{Read, Write};

        let executable = std::env::current_exe().expect("test executable");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../wikitest/scenarios/mechanical/docs-discovery/fixtures/mediawiki.json");
        let (sender, receiver) = std::sync::mpsc::channel();
        let failed = std::thread::spawn(move || {
            let wiki = FakeWiki::from_file(&fixture);
            let project = Project::with_wiki(&executable, &wiki);
            sender
                .send(project.path().to_path_buf())
                .expect("send path");
            let address = wiki
                .api_url()
                .strip_prefix("http://")
                .and_then(|rest| rest.strip_suffix("/api.php"))
                .expect("loopback address")
                .to_owned();
            let mut stream = std::net::TcpStream::connect(&address).expect("connect");
            write!(
                stream,
                "GET /api.php?action=query&lgpassword=hunter2&token=csrf-secret HTTP/1.1\r\n\
                 Host: {address}\r\nConnection: close\r\n\r\n"
            )
            .expect("send request");
            let mut response = Vec::new();
            stream.read_to_end(&mut response).expect("read response");
            panic!("simulated test failure");
        })
        .join();
        assert!(failed.is_err(), "simulated failure did not panic");
        let kept_path = receiver.recv().expect("kept path");
        let written = fs::read_to_string(kept_path.join(FAKE_WIKI_OBSERVATION));
        let _ = fs::remove_dir_all(&kept_path);
        let written = written.expect("observation file was not written");
        let observation: Value = serde_json::from_str(&written).expect("observation JSON");
        let params = &observation["requests"][0]["params"];
        assert_eq!(params["action"], "query", "{written}");
        assert_eq!(params["lgpassword"], "<redacted>", "{written}");
        assert_eq!(params["token"], "<redacted>", "{written}");
        assert!(!written.contains("hunter2") && !written.contains("csrf-secret"));
        assert!(
            observation["pages"]
                .as_array()
                .is_some_and(|pages| !pages.is_empty())
        );
    }
}
