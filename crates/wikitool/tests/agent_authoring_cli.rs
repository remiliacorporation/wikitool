//! Agent-facing authoring workflow: adapter inspection, a neutral interview ledger and its open
//! items, draft review bound to the brief, a changeset accepted by a named editor, promotion
//! from that changeset and post-promotion validation.
//!
//! Ported from `wikitest/scenarios/mechanical/agent-authoring-workflow/scenario.json`; comments
//! name the scenario's step IDs. One test, because the interview brief, changeset and promoted
//! page each build on the previous step.

use std::path::PathBuf;

use wikitest::harness::Project;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/agent-authoring-workflow/fixtures")
        .join(name)
}

const DRAFT: &str = ".wikitool/drafts/Complex_Draft.wiki";
const CHANGESET: &str = ".wikitool/review/complex-draft.json";

#[test]
fn interview_review_changeset_promotion_and_validation() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    // --- Project with an explicit adapter and a draft ---

    // install-adapter
    wt.install(
        fixture("site-adapter.toml"),
        "site-adapter/site-adapter.toml",
    );

    // initialize-project
    wt.run(&[
        "init",
        "--templates",
        "--no-network",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);

    // install-quality-template, install-reflist-template, install-cite-template, install-draft
    wt.install(
        fixture("Template_Article_quality.wiki"),
        "templates/misc/Template_Article_quality.wiki",
    );
    wt.install(
        fixture("Template_Reflist.wiki"),
        "templates/misc/Template_Reflist.wiki",
    );
    wt.install(
        fixture("Template_Cite_web.wiki"),
        "templates/cite/Template_Cite_web.wiki",
    );
    wt.install(fixture("Complex_Draft.wiki"), DRAFT);

    // build-catalog
    wt.run(&["catalog", "build", "--format", "json"])
        .assert_code(0);

    // inspect-adapter
    let adapter = wt.run(&["adapter", "inspect", "--format", "json"]);
    adapter
        .assert_code(0)
        .assert_json("/adapter/adapter_id", "wikitest-wiki");
    adapter.json_str("/publication_identity/policy_sha256");

    // --- Interview ledger and open items ---

    // initialize-interview
    let interview = wt.run(&[
        "interview",
        "init",
        "Complex Draft",
        "--intent",
        "new",
        "--no-scout",
        "--agent",
        "Wikitest agent",
        "--timestamp",
        "20260829T140000Z",
        "--format",
        "json",
    ]);
    interview
        .assert_code(0)
        .assert_json("/schema_version", "wiki_interview_init_v1")
        .assert_json("/scout_included", false);
    let brief = interview.json_str("/brief_path");

    // add-open-item
    wt.run(&[
        "interview",
        "open-item",
        "add",
        &brief,
        "--kind",
        "privacy-exclusion",
        "--text",
        "Do not publish the private donor address.",
        "--item-id",
        "privacy-1",
        "--notes",
        "Fixture exclusion",
        "--timestamp",
        "20260829T140100Z",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/item/item_id", "privacy-1")
    .assert_json("/item/status", "open");

    // list-open-item
    wt.run(&["interview", "open-item", "list", &brief, "--format", "json"])
        .assert_code(0)
        .assert_stdout_contains("Do not publish the private donor address.");

    // resolve-open-item
    wt.run(&[
        "interview",
        "open-item",
        "update",
        &brief,
        "--item-id",
        "privacy-1",
        "--status",
        "resolved",
        "--notes",
        "Excluded from the article brief",
        "--timestamp",
        "20260829T140200Z",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/item/status", "resolved");

    // list-resolved-item
    wt.run(&["interview", "open-item", "list", &brief, "--format", "json"])
        .assert_code(0)
        .assert_stdout_contains("resolved");

    // validate-interview
    wt.run(&["interview", "validate", &brief, "--format", "json"])
        .assert_code(0)
        .assert_json("/status", "warning")
        .assert_json_contains("/summary/sections_unfilled", "Article Object");

    // show-interview
    wt.run(&[
        "interview",
        "show",
        &brief,
        "--format",
        "json",
        "--view",
        "full",
    ])
    .assert_code(0)
    .assert_json("/summary/open_item_counts/resolved", 1);

    // audit-interviews
    wt.run(&["interview", "audit", "--format", "json", "--view", "brief"])
        .assert_code(0)
        .assert_json("/total_briefs", 1);

    // --- Review, changeset acceptance and promotion ---

    // review-draft-with-brief
    let review = wt.run(&[
        "review",
        "--draft-path",
        DRAFT,
        "--title",
        "Complex Draft",
        "--brief-path",
        &brief,
        "--summary",
        "Wikitest draft review",
        "--format",
        "json",
        "--view",
        "full",
    ]);
    review.assert_code(0).assert_json("/filters/mode", "draft");
    review.json_at("/interview_brief");
    review.assert_stdout_contains("promote_draft");

    // prepare-changeset
    wt.run(&[
        "article",
        "changeset",
        "prepare",
        DRAFT,
        "--title",
        "Complex Draft",
        "--output",
        CHANGESET,
        "--prose-origin",
        "collaborative-draft",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/item_count", 1)
    .assert_json("/manifest/items/0/prose_origin", "collaborative_draft");
    wt.assert_file_exists(CHANGESET);

    // accept-changeset
    wt.run(&[
        "article",
        "changeset",
        "accept",
        CHANGESET,
        "--human-editor",
        "Wikitest fixture editor",
        "--warnings",
        "accept",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/item_count", 1)
    .assert_json("/editor_identity_assurance", "self_reported_unverified");

    // promote-draft
    wt.run(&[
        "article",
        "promote",
        DRAFT,
        "--title",
        "Complex Draft",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/prose_origin", "collaborative_draft")
    .assert_json("/source_preserved", true);
    wt.assert_file_contains(
        "wiki_content/Main/Complex_Draft.wiki",
        "fictional archive used to test",
    );

    // validate-promoted-article
    let validate = wt.run(&[
        "validate",
        "--title",
        "Complex Draft",
        "--summary",
        "--advisory",
        "--format",
        "json",
    ]);
    validate.assert_code(0).json_at("/status");
    validate.assert_stdout_lacks("private donor address");
}
