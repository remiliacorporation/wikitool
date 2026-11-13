//! Template contract lifecycle through public commands: contract validation, exact-state
//! scaffold planning and application with replay refusal, observed-contract capture and
//! overwrite refusal, semantic `action=parse` render checks, exact migration planning, and the
//! parameter declarations the brief view reports for Lua, mixed and source templates.
//!
//! Ported from `wikitest/scenarios/mechanical/template-contracts/scenario.json`; comments name
//! the scenario's step IDs. One test, because the scaffold plan ID, the generated template and
//! the template catalog carry from one step to the next, and the parse wire-contract check
//! counts requests of the whole run.

use std::path::PathBuf;

use wikitest::harness::{FakeWiki, Project};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/template-contracts/fixtures")
        .join(name)
}

const CARD: &str = "templates/core/Template_Card.wiki";
const CAPTURE: &str = "contracts/card.capture.json";

#[test]
fn contract_scaffold_capture_render_migration_and_briefs() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), &wiki);

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

    // --- Contract check and plan-bound scaffold ---

    // install-helper-template, install-contract
    wt.install(
        fixture("Template_Helper.wiki"),
        "templates/core/Template_Helper.wiki",
    );
    wt.install(
        fixture("card.contract.json"),
        "contracts/card.contract.json",
    );

    // check-contract
    wt.run(&[
        "templates",
        "contract",
        "check",
        "contracts/card.contract.json",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/status", "clean")
    .assert_json("/compatibility", "new_template");

    // plan-scaffold
    let scaffold_args = [
        "templates",
        "scaffold",
        "contracts/card.contract.json",
        "--output",
        CARD,
    ];
    let plan = wt.run(&[&scaffold_args[..], &["--format", "json"]].concat());
    plan.assert_code(0).assert_json("/mode", "plan");
    let plan_id = plan.json_str("/plan_id");
    wt.assert_missing(CARD);

    // apply-scaffold
    wt.run(
        &[
            &scaffold_args[..],
            &["--apply", &plan_id, "--format", "json"],
        ]
        .concat(),
    )
    .assert_code(0)
    .assert_json("/mode", "applied");
    wt.assert_file_contains(CARD, "<templatedata>");
    wt.assert_file_contains(CARD, "[[Category:Card templates]]");

    // reject-scaffold-replay
    wt.run(
        &[
            &scaffold_args[..],
            &["--apply", &plan_id, "--overwrite", "--format", "json"],
        ]
        .concat(),
    )
    .assert_code(1)
    .assert_stderr_contains("plan_id mismatch");

    // --- Capture the generated template's observed contract ---

    // build-template-catalog
    wt.run(&["templates", "catalog", "build", "--format", "json"])
        .assert_code(0);

    // capture-generated-template
    let capture_args = [
        "templates",
        "contract",
        "capture",
        "Template:Card",
        "--output",
        CAPTURE,
        "--format",
        "json",
    ];
    wt.run(&capture_args)
        .assert_code(0)
        .assert_json("/schema", "template_contract_capture_v1")
        .assert_json("/authority", "observed_starter_not_target_design");
    wt.assert_file_contains(
        CAPTURE,
        r#""schema_version": "template_engineering_contract_v1""#,
    );

    // reject-capture-overwrite
    wt.run(&capture_args)
        .assert_code(1)
        .assert_stderr_contains("refuses existing output");

    // --- Semantic render checks through action=parse ---

    // render-check-contract
    wt.run(&[
        "templates",
        "contract",
        "render-check",
        "contracts/card.contract.json",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/status", "clean")
    .assert_json("/fixture_count", 1);

    // install-bad-contract
    wt.install(
        fixture("card.bad-contract.json"),
        "contracts/card.bad-contract.json",
    );

    // reject-semantic-mismatch
    wt.run(&[
        "templates",
        "contract",
        "render-check",
        "contracts/card.bad-contract.json",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/failed_count", 1)
    .assert_json("/fixtures/0/dom_assertions/1/passed", false);

    // assert-parse-wire-contract
    wiki.assert_requests(
        "POST",
        &[
            ("action", "parse"),
            ("contentmodel", "wikitext"),
            ("disableeditsection", "1"),
            ("disablelimitreport", "1"),
            ("prop", "text|displaytitle"),
            ("title", "Wikitool template fixture"),
        ],
        2,
    );

    // --- Migration planning leaves pages untouched ---

    // install-migration-wiki, install-migration-collision-wiki, install-migration-json
    wt.install(
        fixture("migration.wiki"),
        "wiki_content/Main/Migration.wiki",
    );
    wt.install(
        fixture("migration-collision.wiki"),
        "wiki_content/Main/Migration_collision.wiki",
    );
    wt.install(fixture("migration.json"), "contracts/migration.json");

    // plan-exact-migration
    wt.run(&[
        "templates",
        "migration-plan",
        "contracts/migration.json",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/affected_files", 2)
    .assert_json("/mechanical_patch_count", 2)
    .assert_json("/review_required_files", 1)
    .assert_json("/retirement_ready", false);
    wt.assert_file_contains("wiki_content/Main/Migration.wiki", "{{Old|old=value}}");
    wt.assert_file_contains(
        "wiki_content/Main/Migration_collision.wiki",
        "{{Old|old=left|new=right}}",
    );

    // --- Brief view of declared parameters ---

    // install-lua-template, install-lua-module, install-mixed-template, install-brief-usage
    wt.install(
        fixture("Template_Lua_card.wiki"),
        "templates/core/Template_Lua_card.wiki",
    );
    wt.install(
        fixture("Module_Lua_card.lua"),
        "templates/core/Module_Lua_card.lua",
    );
    wt.install(
        fixture("Template_Mixed_card.wiki"),
        "templates/core/Template_Mixed_card.wiki",
    );
    wt.install(
        fixture("brief-usage.wiki"),
        "wiki_content/Main/Brief_usage.wiki",
    );

    // index-brief-usage
    wt.run(&["catalog", "build", "--format", "json"])
        .assert_code(0);

    // catalog-brief-fixtures
    wt.run(&["templates", "catalog", "build", "--format", "json"])
        .assert_code(0);

    // lua-template-brief
    wt.run(&[
        "templates",
        "show",
        "Template:Lua card",
        "--format",
        "json",
        "--view",
        "brief",
    ])
    .assert_code(0)
    .assert_json("/contract/declared_parameter_count", 3)
    .assert_json("/contract/has_templatedata", true)
    .assert_json("/contract/observed_only_parameters/0/name", "unknown");

    // mixed-template-brief
    wt.run(&[
        "templates",
        "show",
        "Template:Mixed card",
        "--format",
        "json",
        "--view",
        "brief",
    ])
    .assert_code(0)
    .assert_json("/contract/declared_parameter_count", 3)
    .assert_json("/contract/has_templatedata", true);

    // source-template-brief
    wt.run(&[
        "templates",
        "show",
        "Template:Helper",
        "--format",
        "json",
        "--view",
        "brief",
    ])
    .assert_code(0)
    .assert_json("/contract/declared_parameter_count", 1)
    .assert_json("/contract/has_templatedata", false);
}
