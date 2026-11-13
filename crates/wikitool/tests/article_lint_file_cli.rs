//! `article lint PATH` reads any file: an export with the metadata header that
//! `export --format wikitext` writes, a file elsewhere in the project, or a file outside it.
//! The header supplies the title and is not linted as page text; findings keep the file's
//! line numbers. Batch selectors keep their `wiki_content/` scope.

use std::fs;

use serde_json::Value;
use wikitest::harness::Project;

const HEADER: &str = "---\n\
title: \"Lint Target\"\n\
source_url: \"https://wiki.example/wiki/Lint_Target\"\n\
source_domain: \"wiki.example\"\n\
fetched_at: \"2026-01-01T00:00:00Z\"\n\
format: \"wiki\"\n\
revision_id: \"42\"\n\
---\n";
const HEADER_LINES: u64 = 8;
const BODY: &str = "Intro.\n\n== History ==\nText.\n\n== History ==\nMore.\n";

fn issue_lines(report: &Value) -> Vec<u64> {
    report["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .filter_map(|issue| issue.pointer("/span/line").and_then(Value::as_u64))
        .collect()
}

#[test]
fn lint_reads_exports_and_files_outside_the_content_directories() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    wt.run(&["init", "--no-network"]).assert_code(0);
    fs::create_dir_all(wt.path().join("exports")).expect("exports dir");
    fs::write(
        wt.path().join("exports/Lint_Target.wiki"),
        format!("{HEADER}{BODY}"),
    )
    .expect("export");
    let outside = tempfile::tempdir().expect("outside dir");
    let plain = outside.path().join("plain.wiki");
    fs::write(&plain, BODY).expect("plain");
    let exported_outside = outside.path().join("exported.wiki");
    fs::write(&exported_outside, format!("{HEADER}{BODY}")).expect("exported");
    let plain = plain.to_string_lossy().into_owned();
    let exported_outside = exported_outside.to_string_lossy().into_owned();

    // An export inside the project but outside wiki_content/ takes its title from the header.
    let exported = wt.run(&[
        "article",
        "lint",
        "exports/Lint_Target.wiki",
        "--format",
        "json",
    ]);
    exported
        .assert_json("/title", "Lint Target")
        .assert_json("/relative_path", "exports/Lint_Target.wiki")
        .assert_json("/export_header/title", "Lint Target")
        .assert_json("/export_header/revision_id", "42")
        .assert_json("/export_header/line_count", HEADER_LINES);
    wt.run(&["article", "lint", "exports/Lint_Target.wiki"])
        .assert_stdout_contains("export_header.source_url: https://wiki.example/wiki/Lint_Target")
        .assert_stdout_contains("title: Lint Target");

    // A file outside the project needs a title from somewhere, and says so.
    wt.run(&["article", "lint", &plain])
        .assert_code(1)
        .assert_stderr_contains("pass --title TITLE");
    let plain_report = wt.run(&[
        "article",
        "lint",
        &plain,
        "--title",
        "Lint Target",
        "--format",
        "json",
    ]);
    plain_report.assert_json("/title", "Lint Target");
    assert!(
        plain_report.json()["export_header"].is_null(),
        "a file without a header reports none"
    );

    let outside_export = wt.run(&["article", "lint", &exported_outside, "--format", "json"]);
    outside_export.assert_json("/title", "Lint Target");

    // --title wins over the header, which is still set aside.
    wt.run(&[
        "article",
        "lint",
        "exports/Lint_Target.wiki",
        "--title",
        "Other Title",
        "--format",
        "json",
    ])
    .assert_json("/title", "Other Title")
    .assert_json("/export_header/title", "Lint Target");

    // The header is not page text: the same body hashes the same and its findings move down
    // by exactly the header's lines.
    let plain_json = plain_report.json();
    let export_json = outside_export.json();
    assert_eq!(plain_json["content_sha256"], export_json["content_sha256"]);
    // file_sha256 is the whole file, the hash accept compares; it differs only when a
    // header was set aside.
    assert_eq!(plain_json["content_sha256"], plain_json["file_sha256"]);
    assert_ne!(export_json["content_sha256"], export_json["file_sha256"]);
    let plain_lines = issue_lines(&plain_json);
    assert!(
        !plain_lines.is_empty(),
        "the duplicate heading should be reported: {plain_json}"
    );
    assert_eq!(
        issue_lines(&export_json),
        plain_lines
            .iter()
            .map(|line| line + HEADER_LINES)
            .collect::<Vec<_>>()
    );

    // Batch selectors still cover wiki_content/ only.
    wt.run(&["article", "lint", "--path", "exports/Lint_Target.wiki"])
        .assert_code(1)
        .assert_stderr_contains("wiki_content");
}

#[test]
fn a_header_in_a_project_owned_file_is_linted_as_page_text_and_reported() {
    const RULE: &str = "structure.export_header_on_publication_path";
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    wt.run(&["init", "--no-network"]).assert_code(0);
    fs::create_dir_all(wt.path().join("wiki_content/Main")).expect("mirror dir");
    fs::write(
        wt.path().join("wiki_content/Main/Foo.wiki"),
        format!("{HEADER}{BODY}"),
    )
    .expect("mirrored page");

    // The path's title wins over the header's, and the header is page text that
    // push would publish.
    let single = wt.run(&[
        "article",
        "lint",
        "wiki_content/Main/Foo.wiki",
        "--format",
        "json",
    ]);
    single
        .assert_code(1)
        .assert_json("/title", "Foo")
        .assert_json_item("/issues", "/rule_id", RULE)
        .assert_json_item("/issues", "/span/line", 1);
    let single = single.json();
    assert!(single["export_header"].is_null());
    assert_eq!(single["content_sha256"], single["file_sha256"]);

    // Every way of naming the file agrees on title and hash.
    let absolute = wt
        .path()
        .join("wiki_content/Main/Foo.wiki")
        .to_string_lossy()
        .into_owned();
    let by_absolute = wt.run(&["article", "lint", &absolute, "--format", "json"]);
    by_absolute
        .assert_code(1)
        .assert_json("/title", "Foo")
        .assert_json_item("/issues", "/rule_id", RULE);
    assert_eq!(
        by_absolute.json()["content_sha256"],
        single["content_sha256"]
    );
    let batch = wt.run(&[
        "article",
        "lint",
        "--path",
        "wiki_content/Main/Foo.wiki",
        "--format",
        "json",
    ]);
    batch
        .assert_code(1)
        .assert_json("/reports/0/title", "Foo")
        .assert_json_item("/reports/0/issues", "/rule_id", RULE);
    assert_eq!(
        batch.json()["reports"][0]["content_sha256"],
        single["content_sha256"]
    );

    // A draft is promoted byte for byte, so a header there is reported too.
    fs::create_dir_all(wt.path().join(".wikitool/drafts")).expect("drafts dir");
    fs::write(
        wt.path().join(".wikitool/drafts/Foo.wiki"),
        format!("{HEADER}{BODY}"),
    )
    .expect("draft");
    wt.run(&[
        "article",
        "lint",
        ".wikitool/drafts/Foo.wiki",
        "--title",
        "Foo",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/title", "Foo")
    .assert_json_item("/issues", "/rule_id", RULE);
}
