//! Local knowledge workspace without network access: resolved configuration, documentation
//! bundle import, search, context and symbol retrieval, disposable database reset, Cargo-shaped
//! CSV and JSON imports, parser/LSP configuration, Scribunto module lint, adapter-selected docs
//! profiles and documentation removal.
//!
//! Ported from `wikitest/scenarios/mechanical/knowledge-workspace/scenario.json`; comments name
//! the scenario's step IDs. One test, because the reset, re-import, profile and removal steps
//! act on the database and configuration earlier steps created.

use std::path::PathBuf;

use serde_json::json;
use wikitest::harness::Project;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/knowledge-workspace/fixtures")
        .join(name)
}

#[test]
fn docs_imports_parser_config_and_database_reset() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    // --- Generic project and inputs ---

    // initialize-project
    wt.run(&["init", "--templates", "--no-network"])
        .assert_code(0);

    // generic-docs-profile
    wt.run(&["catalog", "status", "--format", "json"])
        .assert_code(0)
        .assert_json("/docs_profile_requested", "mw-1.44-authoring");

    // install-docs-bundle, install-csv, install-json, install-module
    wt.install(fixture("docs-bundle.json"), "inputs/docs-bundle.json");
    wt.install(fixture("cargo-records.csv"), "inputs/cargo-records.csv");
    wt.install(fixture("cargo-records.json"), "inputs/cargo-records.json");
    wt.install(
        fixture("Module_Fixture.lua"),
        "templates/misc/Module_Fixture.lua",
    );

    // show-config
    wt.run(&["config", "show", "--format", "json"])
        .assert_code(0)
        .assert_json("/schema_version", "wikitool_config_v2")
        .assert_json("/wiki/api_url/source", "none");

    // --- Documentation bundle and retrieval ---

    // import-docs
    wt.run(&["docs", "import", "--bundle", "inputs/docs-bundle.json"])
        .assert_code(0)
        .assert_stdout_contains("imported_extensions: 1");

    // list-docs
    wt.run(&["docs", "list", "--format", "json"])
        .assert_code(0)
        .assert_json("/stats/pages_count", 3)
        .assert_json_item("/corpora", "/corpus_id", "extension:fixtureextension");

    // search-docs
    wt.run(&["docs", "search", "AlphaBetaToken", "--format", "json"])
        .assert_code(0)
        .assert_json_item("", "/page_title", "Extension:FixtureExtension");

    // retrieve-docs-context
    wt.run(&["docs", "context", "AlphaBetaToken", "--format", "json"])
        .assert_code(0)
        .assert_json_item("/pages", "/page_title", "Extension:FixtureExtension")
        .assert_json_item("/sections", "/page_title", "Extension:FixtureExtension");

    // retrieve-docs-symbol
    wt.run(&[
        "docs",
        "symbols",
        "$wgFixtureExtensionEnable",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/0/symbol_name", "$wgFixtureExtensionEnable")
    .assert_json("/0/symbol_kind", "config");

    // --- Disposable database reset ---

    // inspect-db
    wt.run(&["db", "stats", "--format", "json"])
        .assert_code(0)
        .assert_json("/db_exists", true)
        .assert_json("/database_schema/status", "ready");

    // reset-db
    wt.run(&["db", "reset", "--yes"])
        .assert_code(0)
        .assert_stdout_contains("deleted: yes");

    // reset-absent-db
    wt.run(&["db", "reset", "--yes"])
        .assert_code(0)
        .assert_stdout_contains("deleted: no");
    wt.assert_missing(".wikitool/data/wikitool.db");

    // reimport-docs-after-reset
    wt.run(&["docs", "import", "--bundle", "inputs/docs-bundle.json"])
        .assert_code(0)
        .assert_stdout_contains("imported_extensions: 1");

    // search-docs-after-reset
    wt.run(&["docs", "search", "BetaToken", "--format", "json"])
        .assert_code(0)
        .assert_json_item("", "/page_title", "Extension:FixtureExtension");

    // --- Cargo-shaped imports ---

    // preview-csv-import
    wt.run(&[
        "import",
        "cargo",
        "inputs/cargo-records.csv",
        "--table",
        "fixture_csv",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json_contains("/pages_created", "CSV Fixture Alpha");
    wt.assert_missing("wiki_content/Main/CSV_Fixture_Alpha.wiki");

    // apply-csv-import
    wt.run(&[
        "import",
        "cargo",
        "inputs/cargo-records.csv",
        "--table",
        "fixture_csv",
        "--write",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json_contains("/pages_created", "CSV Fixture Alpha");
    wt.assert_file_contains(
        "wiki_content/Main/CSV_Fixture_Alpha.wiki",
        "generated import record",
    );

    // apply-json-import
    wt.run(&[
        "import",
        "cargo",
        "inputs/cargo-records.json",
        "--table",
        "fixture_json",
        "--write",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json_contains("/pages_created", "JSON Fixture Gamma");
    wt.assert_file_contains(
        "wiki_content/Main/JSON_Fixture_Gamma.wiki",
        "generated import record",
    );

    // --- Parser configuration and module lint ---

    // generate-lsp-config
    wt.run(&["lsp", "generate-config", "--force"])
        .assert_code(0);
    wt.assert_file_exists(".wikitool/parser-config.json");

    // inspect-lsp-status
    wt.run(&["lsp", "status", "--format", "json"])
        .assert_code(0)
        .assert_json("/parser_config_exists", true)
        .assert_json("/runtime_config_exists", true);

    // lint-module
    wt.run(&[
        "module",
        "lint",
        "Module:Fixture",
        "--format",
        "json",
        "--no-meta",
    ])
    .assert_code(0)
    .assert_json("/inspected_modules", 1)
    .assert_json("/total_errors", 0)
    .assert_json("/results/0/title", "Module:Fixture");

    // --- Adapter-selected docs profile and removal ---

    // install-site-adapter
    wt.install(
        fixture("site-adapter.toml"),
        "site-adapter/site-adapter.toml",
    );

    // select-site-adapter
    wt.run(&[
        "init",
        "--no-network",
        "--force",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);

    // adapter-docs-profile
    wt.run(&["catalog", "status", "--format", "json"])
        .assert_code(0)
        .assert_json("/docs_profile_requested", "mw-1.44-site-authoring");

    // explicit-catalog-profile
    wt.run(&[
        "catalog",
        "status",
        "--docs-profile",
        "mw-1.44-authoring",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/docs_profile_requested", "mw-1.44-authoring");

    // adapter-context-profile
    wt.run(&["docs", "context", "hooks", "--format", "json"])
        .assert_code(0)
        .assert_json("/profile", "mw-1.44-site-authoring");

    // explicit-context-profile
    wt.run(&[
        "docs",
        "context",
        "hooks",
        "--profile",
        "mw-1.44-authoring",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/profile", "mw-1.44-authoring");

    // remove-docs
    wt.run(&["docs", "remove", "FixtureExtension"])
        .assert_code(0);

    // verify-docs-removed
    wt.run(&["docs", "search", "AlphaBetaToken", "--format", "json"])
        .assert_code(0)
        .assert_json("", json!([]));
}
