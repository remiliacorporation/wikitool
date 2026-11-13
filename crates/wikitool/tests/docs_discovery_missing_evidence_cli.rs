//! Installed-extension documentation discovery refuses a `meta=siteinfo` response without
//! `query.extensions` and stops before any documentation request: absent evidence is never an
//! empty installation.
//!
//! Ported from `wikitest/scenarios/mechanical/docs-discovery-missing-evidence/scenario.json`;
//! comments name the scenario's step IDs.

use std::path::PathBuf;
use std::time::Duration;

use wikitest::harness::{FakeWiki, Project};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/docs-discovery-missing-evidence/fixtures")
        .join(name)
}

#[test]
fn installed_docs_refuse_missing_evidence() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), &wiki);

    // initialize-project
    wt.run(&["init", "--no-network", "--api-url", wiki.api_url()])
        .assert_code(0);

    // import-installed-docs
    wt.set_timeout(Duration::from_millis(5000));
    wt.run(&["docs", "import", "--installed", "--no-subpages"])
        .assert_code(1)
        .assert_stderr_contains(
            "installed-extensions response requires query.extensions with names and types",
        );

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
        0,
    );
    wiki.assert_requests(
        "GET",
        &[("action", "query"), ("titles", "Extension:MinervaNeue")],
        0,
    );
    wiki.assert_requests("GET", &[], 1);
    wiki.assert_requests("POST", &[], 0);
}
