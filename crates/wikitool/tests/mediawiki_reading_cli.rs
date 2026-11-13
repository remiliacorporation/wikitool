//! Read-only MediaWiki discovery and rendering against a deterministic fake wiki: configured
//! text and title search, remote and stored capability discovery, rendered scope validation,
//! and refusal to export from a non-public address.
//!
//! Ported from `wikitest/scenarios/mechanical/mediawiki-reading/scenario.json`; comments name
//! the scenario's step IDs. One test, because the stored-capability steps depend on each other
//! and the final wire-contract check counts the requests of the whole run.

use std::path::PathBuf;

use wikitest::harness::{FakeWiki, Project};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/mediawiki-reading/fixtures")
        .join(name)
}

#[test]
fn search_capabilities_render_check_and_export_guard() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), &wiki);
    // The JSON runner's {MEDIAWIKI_ORIGIN_URL}: the API URL without its `/api.php`.
    let origin = wiki
        .api_url()
        .strip_suffix("/api.php")
        .expect("fake wiki endpoint ends in /api.php");

    // initialize-project
    wt.run(&[
        "init",
        "--templates",
        "--no-network",
        "--wiki-url",
        origin,
        "--api-url",
        wiki.api_url(),
    ])
    .assert_code(0);

    // --- Search ---

    // search-page-content
    wt.run(&[
        "source",
        "wiki-search",
        "ephemera",
        "--limit",
        "5",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/query", "ephemera")
    .assert_json("/total_hits", 1)
    .assert_json_item("/hits", "/title", "Aster Archive");

    // search-page-title
    wt.run(&[
        "source",
        "wiki-search",
        "Aster Item",
        "--what",
        "title",
        "--limit",
        "5",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/what", "title")
    .assert_json_item("/hits", "/title", "Aster Item");

    // --- Capabilities: remote probe, then the stored manifest ---

    // probe-remote-capabilities
    wt.run(&[
        "wiki",
        "capabilities",
        "remote",
        wiki.api_url(),
        "--format",
        "json",
        "--view",
        "summary",
    ])
    .assert_code(0)
    .assert_json("/schema_version", "remote_wiki_capabilities_v1")
    .assert_json("/storage", "not_stored")
    .assert_json("/capabilities/mediawiki_version", "1.44.6");

    // show-absent-capabilities
    wt.run(&[
        "wiki",
        "capabilities",
        "show",
        "--format",
        "json",
        "--view",
        "summary",
    ])
    .assert_code(1)
    .assert_stderr_contains("wiki capability manifest is missing");

    // sync-capabilities
    wt.run(&[
        "wiki",
        "capabilities",
        "sync",
        "--format",
        "json",
        "--view",
        "full",
    ])
    .assert_code(0)
    .assert_json("/schema_version", "wiki_capabilities_v2")
    .assert_json("/has_scribunto", true);

    // show-capabilities
    wt.run(&[
        "wiki",
        "capabilities",
        "show",
        "--format",
        "json",
        "--view",
        "summary",
    ])
    .assert_code(0)
    .assert_json("/mediawiki_version", "1.44.6")
    .assert_json("/has_scribunto", true);

    // --- Rendering and export ---

    // render-check-page
    wt.run(&[
        "wiki",
        "render-check",
        "Aster Archive",
        "--scope-class",
        "archive-card",
        "--expect-scopes",
        "1",
        "--require-interactive-link",
        "--require-href-contains",
        "Aster_Item",
        "--require-link-class",
        "archive-link",
        "--format",
        "json",
        "--view",
        "full",
    ])
    .assert_code(0)
    .assert_json("/status", "clean")
    .assert_json("/scope_count", 1);

    // reject-private-export
    wt.run(&[
        "export",
        &format!("{origin}/wiki/Aster_Archive"),
        "--output",
        "exports/Aster_Archive.wiki",
        "--format",
        "wikitext",
        "--no-frontmatter",
    ])
    .assert_code(1)
    .assert_stderr_contains("source URL host resolves to a non-public address");
    wt.assert_missing("exports/Aster_Archive.wiki");

    // --- assert-read-wire-contract ---

    let search = [("action", "query"), ("list", "search")];
    wiki.assert_requests(
        "GET",
        &[&search[..], &[("srsearch", "ephemera"), ("srwhat", "text")]].concat(),
        1,
    );
    wiki.assert_requests(
        "GET",
        &[
            &search[..],
            &[("srsearch", "Aster Item"), ("srwhat", "title")],
        ]
        .concat(),
        1,
    );
    wiki.assert_requests(
        "GET",
        &[
            ("action", "parse"),
            ("page", "Aster Archive"),
            ("prop", "text|displaytitle|revid"),
        ],
        1,
    );
}
