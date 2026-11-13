//! Indexed local catalog retrieval: catalog readiness before and after a build, index stats,
//! cross-page chunk retrieval, link backlinks and the article scout, through public JSON output.
//!
//! Ported from `wikitest/scenarios/catalog/indexed-retrieval/scenario.json`; comments name the
//! scenario's step IDs. One test, because every query reads the index built from the same
//! installed pages.

use std::path::PathBuf;

use serde_json::json;
use wikitest::harness::Project;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/catalog/indexed-retrieval/fixtures")
        .join(name)
}

#[test]
fn catalog_build_backlinks_chunks_and_scout() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    // --- Install a small wiki and check catalog readiness around the build ---

    // initialize-project
    wt.run(&["init", "--templates", "--no-network"])
        .assert_code(0);

    // install-alpha, install-beta, install-reflist-template, install-cite-template
    wt.install(
        fixture("Alpha.wiki"),
        "wiki_content/Main/Alpha_Archive.wiki",
    );
    wt.install(
        fixture("Beta_Index.wiki"),
        "wiki_content/Main/Beta_Index.wiki",
    );
    wt.install(
        fixture("Template_Reflist.wiki"),
        "templates/misc/Template_Reflist.wiki",
    );
    wt.install(
        fixture("Template_Cite_web.wiki"),
        "templates/cite/Template_Cite_web.wiki",
    );

    // inspect-unbuilt-catalog
    wt.run(&["catalog", "status", "--format", "json"])
        .assert_code(0)
        .assert_json("/readiness", "not_ready")
        .assert_json_contains("/degradations", "content_index_missing")
        .assert_json_contains("/degradations", "docs_profile_missing");

    // build-index
    wt.run(&["catalog", "build", "--format", "json"])
        .assert_code(0)
        .assert_json_at_least("/rebuild/inserted_rows", 4)
        .assert_json("/status/readiness", "content_ready")
        .assert_json("/status/content_index_ready", true);
    wt.assert_file_exists(".wikitool/data/wikitool.db");

    // inspect-built-catalog
    wt.run(&["catalog", "status", "--format", "json"])
        .assert_code(0)
        .assert_json("/readiness", "content_ready")
        .assert_json("/degradations", json!(["docs_profile_missing"]));

    // inspect-stats
    wt.run(&["catalog", "inspect", "stats", "--format", "json"])
        .assert_code(0)
        .assert_json("/index_ready", true)
        .assert_json_at_least("/content_index/indexed_rows", 4)
        .assert_json_at_least("/scan/total_files", 4);

    // --- Retrieval from the built index ---

    // retrieve-across-pages
    wt.run(&[
        "catalog",
        "inspect",
        "chunks",
        "--across-pages",
        "--query",
        "bulletins archive",
        "--limit",
        "4",
        "--max-pages",
        "2",
        "--token-budget",
        "240",
        "--format",
        "json",
        "--view",
        "brief",
    ])
    .assert_code(0)
    .assert_json("/schema_version", "wikitool_brief_v1")
    .assert_json("/status", "found")
    .assert_json_at_least("/source_page_count", 1)
    .assert_json_item("/chunks", "/source_title", "Alpha Archive");

    // inspect-backlinks
    wt.run(&[
        "catalog",
        "inspect",
        "backlinks",
        "Beta Index",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/storage_ready", true)
    .assert_json("/title", "Beta Index")
    .assert_json_contains("/backlinks", "Alpha Archive");

    // assemble-article-scout
    let scout = wt.run(&[
        "article",
        "scout",
        "Gamma Catalogue",
        "--intent",
        "new",
        "--format",
        "json",
        "--view",
        "brief",
    ]);
    scout
        .assert_code(0)
        .assert_json("/schema_version", "wikitool_brief_v1")
        .assert_json("/command", "article scout")
        .assert_json("/local_state", "linked_but_missing")
        .assert_json_at_least("/context/backlink_count", 1)
        .json_at("/context");
    scout
        .assert_stdout_lacks(r#""article_type""#)
        .assert_stdout_lacks(r#""confidence""#);
}
