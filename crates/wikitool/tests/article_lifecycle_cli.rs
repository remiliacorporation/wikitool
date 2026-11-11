//! Mechanical article lifecycle in an isolated project: explicit adapter initialization,
//! deterministic lint and safe fixes, the exact-content acceptance store, draft promotion, and
//! refusal to promote a draft that changed after acceptance.
//!
//! Ported from `wikitest/scenarios/mechanical/article-lifecycle/scenario.json`; comments name
//! the scenario's step IDs. One test, because each stage acts on the draft the previous stage
//! left behind.

use std::path::PathBuf;

use wikitest::harness::Project;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/article-lifecycle/fixtures")
        .join(name)
}

const DRAFT: &str = ".wikitool/drafts/Example_Draft.wiki";
const PROMOTED: &str = "wiki_content/Main/Example_Draft.wiki";
const ACCEPTED_SHA256: &str = "c9c8787e616a883af5913ff71b17bd19b1e364fe957c01cfb5c0fef115a61cfe";

#[test]
fn lint_fix_accept_promote_and_refuse_stale_acceptance() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    // --- Initialize generically, then with an explicit site adapter ---

    // initialize-generic
    wt.run(&[
        "init",
        "--templates",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--no-network",
    ])
    .assert_code(0);
    wt.assert_file_exists(".wikitool/config.toml");
    wt.assert_file_exists(".wikitool/parser-config.json");

    // install-adapter
    wt.install(
        fixture("site-adapter.toml"),
        "site-adapter/site-adapter.toml",
    );

    // initialize-adapter
    wt.run(&[
        "init",
        "--templates",
        "--no-network",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--force",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);
    wt.assert_file_contains(".wikitool/config.toml", "site-adapter/site-adapter.toml");

    // install-quality-template, install-reflist-template, install-cite-template,
    // install-broken-draft
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
    wt.install(fixture("broken-draft.wiki"), DRAFT);

    // --- Lint and safe fixes ---

    // lint-broken-draft
    wt.run(&[
        "article",
        "lint",
        DRAFT,
        "--title",
        "Example Draft",
        "--strict",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/schema_version", "article_lint_v3")
    .assert_json_at_least("/errors", 1)
    .assert_json_item("/issues", "/rule_id", "structure.markdown_heading")
    .assert_json_item("/issues", "/rule_id", "structure.require_reflist")
    .assert_json_item("/issues", "/rule_id", "citation.after_punctuation")
    .assert_json_item("/issues", "/rule_id", "citation.source_review")
    .assert_stderr_contains("article lint found");

    // apply-safe-fixes
    wt.run(&[
        "article",
        "fix",
        DRAFT,
        "--title",
        "Example Draft",
        "--apply",
        "safe",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/schema_version", "article_fix_v2")
    .assert_json("/changed", true)
    .assert_json_at_least("/applied_fix_count", 3)
    .assert_json("/remaining_report/errors", 0);
    wt.assert_file_contains(DRAFT, "== History ==");
    wt.assert_file_contains(DRAFT, "{{Reflist}}");
    wt.assert_file_contains(DRAFT, "scenario.<ref>");

    // lint-fixed-draft
    wt.run(&[
        "article",
        "lint",
        DRAFT,
        "--title",
        "Example Draft",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/errors", 0)
    .assert_json_item("/issues", "/rule_id", "citation.source_review");

    // --- Accept the exact bytes and promote them ---

    // record-fixture-acceptance
    wt.run(&[
        "article",
        "accept",
        DRAFT,
        "--title",
        "Example Draft",
        "--human-editor",
        "Wikitest fixture identity",
        "--prose-origin",
        "agent-draft",
        "--allow-warnings",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/schema_version", "article_accept_v3")
    .assert_json("/decision", "accepted_for_main_namespace_promotion")
    .assert_json("/editor_identity_assurance", "self_reported_unverified")
    .assert_json("/prose_origin", "agent_draft")
    .assert_json("/content_sha256", ACCEPTED_SHA256)
    .assert_json(
        "/publication_authority/target_api_url",
        "https://wikitest.invalid/api.php",
    )
    .assert_json("/publication_authority/site_adapter_id", "wikitest-wiki")
    .assert_json(
        "/publication_authority/publication_policy_sha256",
        "1f281c51187b9b8c34b9ab95cdbf45a43b37303311f2db3cc0130bf6e571cd69",
    )
    .assert_json("/warnings_explicitly_accepted", true);

    // promote-accepted-draft
    wt.run(&[
        "article",
        "promote",
        DRAFT,
        "--title",
        "Example Draft",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/schema_version", "article_promote_v4")
    .assert_json("/source_preserved", true)
    .assert_json("/editor_identity_assurance", "self_reported_unverified")
    .assert_json("/prose_origin", "agent_draft")
    .assert_json("/content_sha256", ACCEPTED_SHA256);
    wt.assert_file_exists(PROMOTED);
    wt.assert_file_contains(
        PROMOTED,
        "fictional archive used for an isolated Wikitest scenario",
    );

    // --- A draft changed after acceptance is not promoted ---

    // mutate-accepted-draft
    wt.install(fixture("mutated-draft.wiki"), DRAFT);

    // reject-stale-acceptance
    wt.run(&[
        "article",
        "promote",
        DRAFT,
        "--title",
        "Example Draft",
        "--overwrite",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_stderr_contains("changed after the recorded acceptance decision");
    wt.assert_file_contains(
        PROMOTED,
        "fictional archive used for an isolated Wikitest scenario",
    );
    wt.assert_file_lacks(PROMOTED, "deliberately changed");
}
