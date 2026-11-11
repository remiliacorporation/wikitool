//! Catalog structural and reference audit over a varied local corpus: reference identity and
//! duplicates, orphans, empty categories, template usage and implementation discovery, and
//! agent-facing authoring-contract search and planning.
//!
//! Ported from `wikitest/scenarios/catalog/structural-audit/scenario.json`; comments name the
//! scenario's step IDs. One test, because every query reads the same built catalog.

use std::path::PathBuf;

use wikitest::harness::Project;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/catalog/structural-audit/fixtures")
        .join(name)
}

#[test]
fn references_orphans_categories_templates_and_contracts() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    // --- Install the corpus and build the catalog ---

    // initialize-project
    wt.run(&["init", "--templates", "--no-network"])
        .assert_code(0);

    // install-alpha, install-beta, install-gamma, install-archivists-category,
    // install-empty-category, install-infobox, install-infobox-doc, install-infobox-module,
    // install-infobox-redirect, install-reflist, install-cite-web
    for (source, target) in [
        (
            "Alpha_Archivist.wiki",
            "wiki_content/Main/Alpha_Archivist.wiki",
        ),
        ("Beta_Index.wiki", "wiki_content/Main/Beta_Index.wiki"),
        ("Gamma_Orphan.wiki", "wiki_content/Main/Gamma_Orphan.wiki"),
        (
            "Category_Archivists.wiki",
            "wiki_content/Category/Archivists.wiki",
        ),
        (
            "Category_Empty_fixture.wiki",
            "wiki_content/Category/Empty_fixture.wiki",
        ),
        (
            "Template_Infobox_person.wiki",
            "templates/infobox/Template_Infobox_person.wiki",
        ),
        (
            "Template_Infobox_person___doc.wiki",
            "templates/infobox/Template_Infobox_person___doc.wiki",
        ),
        (
            "Module_Infobox_person.lua",
            "templates/infobox/Module_Infobox_person.lua",
        ),
        (
            "Template_Infobox_human.wiki",
            "templates/infobox/_redirects/Template_Infobox_human.wiki",
        ),
        (
            "Template_Reflist.wiki",
            "templates/misc/Template_Reflist.wiki",
        ),
        (
            "Template_Cite_web.wiki",
            "templates/cite/Template_Cite_web.wiki",
        ),
    ] {
        wt.install(fixture(source), target);
    }

    // build-catalog
    wt.run(&["catalog", "build", "--format", "json"])
        .assert_code(0)
        .assert_json("/status/content_index_ready", true)
        .assert_json_at_least("/rebuild/inserted_rows", 10);

    // --- References ---

    // reference-summary
    wt.run(&[
        "catalog",
        "inspect",
        "references",
        "summary",
        "--all",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json_at_least("/report/reference_count", 3);

    // reference-list
    wt.run(&[
        "catalog",
        "inspect",
        "references",
        "list",
        "--domain",
        "sources.invalid",
        "--all",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_stdout_contains("https://sources.invalid/aster-record");

    // reference-duplicates
    wt.run(&[
        "catalog",
        "inspect",
        "references",
        "duplicates",
        "--all",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_stdout_contains("https://sources.invalid/aster-record");

    // --- Link structure ---

    // inspect-orphans
    wt.run(&["catalog", "inspect", "orphans", "--format", "json"])
        .assert_code(0)
        .assert_json_contains("/titles", "Gamma Orphan");

    // inspect-empty-categories
    wt.run(&["catalog", "inspect", "empty-categories", "--format", "json"])
        .assert_code(0)
        .assert_json_contains("/titles", "Category:Empty fixture");

    // --- Templates and authoring contracts ---

    // inspect-template-catalog
    wt.run(&[
        "catalog",
        "inspect",
        "templates",
        "--all",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_stdout_contains("Template:Infobox person");

    // inspect-template-detail
    wt.run(&[
        "catalog",
        "inspect",
        "templates",
        "Infobox person",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_stdout_contains("module")
    .assert_stdout_contains("documentation");

    // search-authoring-contracts
    wt.run(&[
        "catalog",
        "contracts",
        "search",
        "person occupation",
        "--profile",
        "author",
        "--limit",
        "5",
        "--token-budget",
        "500",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_stdout_contains("Infobox person");

    // plan-authoring-contracts
    wt.run(&[
        "catalog",
        "contracts",
        "plan",
        "Alpha Archivist",
        "--profile",
        "author",
        "--limit",
        "5",
        "--token-budget",
        "500",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/query", "Alpha Archivist")
    .assert_stdout_contains("Infobox person");
}
