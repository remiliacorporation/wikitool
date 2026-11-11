use wikitest::harness::{Output, Project};

#[test]
fn import_rejects_literal_cookie_argv_without_echoing_it() {
    let wt = initialized_project();
    let literal = "session_cookie=literal-argv-sentinel";

    let output = wt.run(&[
        "source",
        "session",
        "import",
        "https://example.invalid/source",
        "--cookies",
        literal,
        "--format",
        "json",
    ]);
    let rendered = combined_output(&output);

    assert!(output.code != Some(0), "literal cookie argv was accepted");
    assert!(
        rendered.contains("existing regular file"),
        "rejection did not explain the safe input contract"
    );
    assert!(
        !rendered.contains(literal) && !rendered.contains("literal-argv-sentinel"),
        "rejection echoed cookie material"
    );
}

#[test]
fn import_accepts_stdin_without_printing_cookie_values() {
    let wt = initialized_project();
    let cookie = "session_cookie=stdin-value-sentinel";
    let output = wt.run_with_stdin(
        &[
            "source",
            "session",
            "import",
            "https://example.invalid/source",
            "--cookies",
            "-",
            "--format",
            "json",
        ],
        cookie.as_bytes(),
    );
    let rendered = combined_output(&output);

    assert!(output.code == Some(0), "stdin session import failed");
    assert!(
        !rendered.contains(cookie) && !rendered.contains("stdin-value-sentinel"),
        "session import output printed cookie material"
    );
    let payload = output.json();
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["session"]["cookie_names"][0], "session_cookie");
}

#[test]
fn import_help_exposes_only_file_or_stdin_inputs() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let output = wt.run(&["source", "session", "import", "--help"]);
    let rendered = combined_output(&output);

    assert!(output.code == Some(0), "session import help failed");
    assert!(rendered.contains("existing regular"));
    assert!(rendered.contains("non-symlink file"));
    assert!(rendered.contains("literal values are rejected"));
    assert!(!rendered.contains("COOKIE_HEADER"));
    assert!(!rendered.contains("literal Cookie header"));
}

fn initialized_project() -> Project<'static> {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let output = wt.run(&[
        "init",
        "--api-url",
        "https://example.invalid/api.php",
        "--no-network",
    ]);
    assert!(output.code == Some(0), "isolated project init failed");
    wt
}

fn combined_output(output: &Output) -> String {
    format!("{}\n{}", output.stdout, output.stderr)
}
