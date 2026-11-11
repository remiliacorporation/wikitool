//! Installed-extension documentation discovery follows registered extension types, excludes
//! skins and imports a repeated extension name once, through the observed loopback API.
//!
//! Ported from `wikitest/scenarios/mechanical/docs-discovery/scenario.json`; comments name the
//! scenario's step IDs. The two refusal cases have their own files and fixtures.

use std::path::PathBuf;
use std::time::Duration;

use wikitest::harness::{FakeWiki, Project};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/docs-discovery/fixtures")
        .join(name)
}

#[test]
fn installed_docs_follow_extension_types_and_deduplicate_names() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), &wiki);

    // initialize-project
    wt.run(&["init", "--no-network", "--api-url", wiki.api_url()])
        .assert_code(0);

    // import-installed-docs
    wt.set_timeout(Duration::from_millis(5000));
    wt.run(&["docs", "import", "--installed", "--no-subpages"])
        .assert_code(0)
        .assert_stdout_contains("imported_extensions: 1");

    // observe-requests
    let siteinfo = [
        ("action", "query"),
        ("meta", "siteinfo"),
        ("siprop", "extensions"),
    ];
    wiki.assert_requests("GET", &siteinfo, 1);
    wiki.assert_requests(
        "GET",
        &[("action", "query"), ("titles", "Extension:Example")],
        1,
    );
    wiki.assert_requests(
        "GET",
        &[("action", "query"), ("titles", "Extension:MinervaNeue")],
        0,
    );
    wiki.assert_requests("GET", &[], 2);
    wiki.assert_requests("POST", &[], 0);
}
