//! `article scout` checks the topic against the configured wiki and reports
//! `live_existence` as exists, redirect, missing, or unchecked with the reason, so a
//! fresh project's local index is never mistaken for the live wiki.

use std::path::PathBuf;

use wikitest::harness::{FakeWiki, Project};
use wikitest::model::MediaWikiPage;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/mediawiki-sync/fixtures")
        .join(name)
}

fn page(title: &str, page_id: i64, content: &str) -> MediaWikiPage {
    MediaWikiPage {
        title: title.to_string(),
        namespace: 0,
        page_id,
        revision_id: page_id * 10,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        content: content.to_string(),
    }
}

#[test]
fn scout_reports_live_existence_from_the_configured_wiki() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    wiki.set_page(page("Radbro", 599, "Radbro is a page.\n"));
    wiki.set_page(page("Radbro alias", 600, "#REDIRECT [[Radbro]]\n"));
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

    // A fresh project has no local copy, yet scout knows the page exists live.
    wt.run(&[
        "article", "scout", "Radbro", "--format", "json", "--view", "full",
    ])
    .assert_json("/live_existence/status", "exists")
    .assert_json("/live_existence/title", "Radbro")
    .assert_json("/live_existence/revision_id", 5990);
    wt.run(&["article", "scout", "Radbro alias", "--format", "json"])
        .assert_json("/live_existence/status", "redirect")
        .assert_json("/live_existence/target", "Radbro");
    wt.run(&["article", "scout", "Radbro Webring", "--format", "json"])
        .assert_json("/live_existence/status", "missing");
    wt.run(&["article", "scout", "Radbro", "--format", "text"])
        .assert_stdout_contains("live_existence: exists (Radbro)");
    wt.run(&[
        "article",
        "scout",
        "Radbro",
        "--no-live-check",
        "--format",
        "json",
    ])
    .assert_json("/live_existence/status", "unchecked")
    .assert_stdout_contains("--no-live-check");
}

#[test]
fn scout_says_unchecked_when_the_wiki_is_unconfigured_or_unreachable() {
    let unconfigured = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    unconfigured.run(&["init", "--no-network"]).assert_code(0);
    unconfigured
        .run(&["article", "scout", "Radbro", "--format", "json"])
        .assert_json("/live_existence/status", "unchecked")
        .assert_stdout_contains("not configured");

    let unreachable = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    unreachable
        .run(&[
            "init",
            "--no-network",
            "--wiki-url",
            "http://127.0.0.1:9",
            "--api-url",
            "http://127.0.0.1:9/api.php",
        ])
        .assert_code(0);
    unreachable
        .run(&["article", "scout", "Radbro", "--format", "json"])
        .assert_json("/live_existence/status", "unchecked")
        .assert_stdout_contains("failed");
}
