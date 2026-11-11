//! Capability-bound template dependency closure: export refuses absent runtime evidence,
//! capabilities are regenerated through the public read-only API, the v2 closure records
//! runtime, missing and unresolved dependencies, and module documentation pages are not
//! counted as modules.
//!
//! Ported from `wikitest/scenarios/mechanical/template-closure/scenario.json`; comments name the
//! scenario's step IDs. One test, because the closure export depends on the capabilities synced
//! after the refusal, and the module surface reads the same catalog.

use std::path::PathBuf;

use serde_json::json;
use wikitest::harness::{FakeWiki, Project};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/template-closure/fixtures")
        .join(name)
}

const CLOSURE: &str = ".wikitool/closures/root.json";

#[test]
fn closure_requires_capabilities_and_records_dependency_evidence() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), &wiki);

    // --- A template catalog without capability evidence ---

    // initialize-project
    wt.run(&[
        "init",
        "--templates",
        "--no-network",
        "--wiki-url",
        "http://127.0.0.1",
        "--api-url",
        wiki.api_url(),
    ])
    .assert_code(0);

    // install-root-template, install-root-module
    wt.install(
        fixture("Template_Root.wiki"),
        "templates/core/Template_Root.wiki",
    );
    wt.install(fixture("Module_Root.lua"), "templates/core/Module_Root.lua");

    // build-template-catalog
    wt.run(&["templates", "catalog", "build", "--format", "json"])
        .assert_code(0);

    // reject-absent-capabilities
    wt.run(&[
        "templates",
        "closure",
        "Root",
        "--output",
        ".wikitool/closures/absent.json",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_stderr_contains("wiki capability manifest is missing")
    .assert_stderr_contains("wikitool wiki capabilities sync");
    wt.assert_missing(".wikitool/closures/absent.json");

    // --- Sync capabilities, then export the closure ---

    // sync-v2-capabilities
    wt.run(&["wiki", "capabilities", "sync", "--format", "json"])
        .assert_code(0)
        .assert_json("/schema_version", "wiki_capabilities_v2")
        .assert_json("/mediawiki_version", "1.44.6")
        .assert_json("/has_scribunto", true)
        .assert_json("/magic_word_count", 1);

    // assert-capability-wire-contract
    let siteinfo = [("action", "query"), ("meta", "siteinfo")];
    wiki.assert_requests(
        "GET",
        &[&siteinfo[..], &[("siprop", "magicwords")]].concat(),
        1,
    );
    wiki.assert_requests(
        "GET",
        &[&siteinfo[..], &[("siprop", "functionhooks")]].concat(),
        1,
    );

    // export-v2-closure
    wt.run(&[
        "templates",
        "closure",
        "Root",
        "--output",
        CLOSURE,
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/schema", "template_dependency_closure_write_v2")
    .assert_json("/runtime_dependency_count", 5)
    .assert_json("/missing_count", 1)
    .assert_json("/unresolved_count", 2);
    for needle in [
        r#""schema_version": "template_dependency_closure_v2""#,
        r#""capability_schema_version": "wiki_capabilities_v2""#,
        r#""capability_manifest_sha256""#,
        r#""kind": "mediawiki_magic_word""#,
        r#""kind": "scribunto_runtime_library""#,
        r#""title": "Module:Missing""#,
        "loads_data_argument_is_not_a_string_literal",
        "parser_function_not_in_capability_manifest:#notRegistered",
    ] {
        wt.assert_file_contains(CLOSURE, needle);
    }

    // --- Module documentation is not a module ---

    // install-module-documentation, install-legacy-documentation-path
    wt.install(
        fixture("module-doc.wiki"),
        "templates/core/Module_Root/doc.wiki",
    );
    wt.install(
        fixture("module-doc.wiki"),
        "templates/core/Module_Legacy/doc.lua",
    );

    // module-surface-excludes-documentation
    wt.run(&[
        "catalog", "surface", "show", "--format", "json", "--view", "full",
    ])
    .assert_code(0)
    .assert_json("/module_count_total", 1)
    .assert_json("/modules/0/module_title", "Module:Root")
    .assert_json("/modules/0/exported_functions", json!([]));
}
