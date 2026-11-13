//! Read-only sync inspection without the global baseline: `pull --title` records a baseline
//! for exactly the named pages, `status` and `diff` compare those titles and label their
//! scope `pulled_titles`, a local file without a recorded baseline is reported as unbaselined
//! rather than new, and `push` still refuses until `pull --full --all` establishes the global
//! baseline.

use std::fs;
use std::path::PathBuf;

use serde_json::json;
use wikitest::harness::{FakeWiki, Project};
use wikitest::model::MediaWikiPage;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/mediawiki-sync/fixtures")
        .join(name)
}

#[test]
fn single_title_pull_unlocks_read_only_status_and_diff_but_not_push() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    wiki.set_page(MediaWikiPage {
        title: "Alpha".to_string(),
        namespace: 0,
        page_id: 900,
        revision_id: 9001,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        content: "Alpha v1\n".to_string(),
    });
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), &wiki);
    wt.run(&[
        "init",
        "--no-network",
        "--wiki-url",
        "http://127.0.0.1",
        "--api-url",
        wiki.api_url(),
    ])
    .assert_code(0);

    // Before any pull there is nothing to compare; the refusal names both routes.
    wt.run(&["status"])
        .assert_code(1)
        .assert_stderr_contains("pull --title TITLE")
        .assert_stderr_contains("pull --full --all");

    let pull = wt.run(&[
        "pull",
        "--title",
        "Alpha",
        "--title",
        "Not yet written",
        "--format",
        "json",
    ]);
    pull.assert_code(0)
        .assert_json("/report/success", true)
        .assert_json("/report/global_baseline_established", false)
        .assert_json("/titles", json!(["Alpha", "Not yet written"]));
    pull.assert_json_item("/report/pages", "/action", "created")
        .assert_json_item("/report/pages", "/action", "missing_remote");
    wt.assert_file_contains("wiki_content/Main/Alpha.wiki", "Alpha v1");

    // The old refusal is gone: status works on the pulled title and says what it covered.
    wt.run(&["status", "--format", "json"])
        .assert_code(0)
        .assert_json("/plan/scope/kind", "pulled_titles")
        .assert_json("/plan/scope/baselined_titles", 1)
        .assert_json("/plan/changes", json!([]));

    fs::write(wt.path().join("wiki_content/Main/Alpha.wiki"), "Alpha v2\n").expect("edit Alpha");
    fs::write(wt.path().join("wiki_content/Main/Beta.wiki"), "Beta\n").expect("write Beta");

    wt.run(&["status", "--format", "json"])
        .assert_code(0)
        .assert_json("/plan/modified_local", 1)
        .assert_json("/plan/new_local", 0)
        .assert_json_item("/plan/scope/unbaselined_local", "/title", "Beta");
    wt.run(&["status"])
        .assert_code(0)
        .assert_stdout_contains("status.scope: pulled_titles")
        .assert_stdout_contains("status.scope.unbaselined: title=Beta")
        .assert_stdout_contains("status.change: type=modified_local title=Alpha");

    wt.run(&["diff", "--content", "--title", "Alpha"])
        .assert_code(0)
        .assert_stdout_contains("diff.scope: pulled_titles")
        .assert_stdout_contains("-Alpha v1")
        .assert_stdout_contains("+Alpha v2");

    // Writes keep requiring the global baseline.
    wt.run(&["push", "--summary", "Scoped edit", "--title", "Alpha"])
        .assert_code(1)
        .assert_stderr_contains("no authoritative global baseline");
    wt.run(&["review", "--summary", "Scoped edit", "--title", "Alpha"])
        .assert_code(1)
        .assert_stderr_contains("no authoritative global baseline");

    // Once the global baseline exists, a file without a row is new again.
    fs::write(wt.path().join("wiki_content/Main/Alpha.wiki"), "Alpha v1\n").expect("restore Alpha");
    wt.run(&["pull", "--full", "--all", "--format", "json"])
        .assert_code(0)
        .assert_json("/report/global_baseline_established", true);
    wt.run(&["status", "--format", "json"])
        .assert_code(0)
        .assert_json("/plan/scope/kind", "global")
        .assert_json("/plan/scope/unbaselined_local", json!([]))
        .assert_json_item("/plan/changes", "/title", "Beta");
}
