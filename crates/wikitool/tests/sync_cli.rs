//! Revision-bound MediaWiki synchronization through Wikitool's public HTTP boundary: pull,
//! redirects, local diff, preview-bound push and delete plans, authenticated one-shot writes,
//! durable mutation reconciliation and operator closure.
//!
//! Ported from `wikitest/scenarios/mechanical/mediawiki-sync/scenario.json`; comments name the
//! scenario's step IDs. It is one test because every section continues the same project and
//! wiki: later plans depend on earlier pulls, and the final wire-contract check counts the
//! requests of the whole run.

use std::fs;
use std::path::PathBuf;

use serde_json::json;
use wikitest::harness::{FakeWiki, Project};
use wikitest::model::MediaWikiPage;

const BOT: &[(&str, &str)] = &[
    ("WIKITOOL_BOT_USER", "wikitest-bot"),
    ("WIKITOOL_BOT_PASS", "wikitest-secret"),
];

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/mediawiki-sync/fixtures")
        .join(name)
}

#[test]
fn revision_bound_sync_push_delete_and_mutation_closure() {
    let wiki = FakeWiki::from_file(fixture("mediawiki.json"));
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), &wiki);

    // --- Pull the remote templates into a fresh project ---

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
    wt.assert_file_contains(".wikitool/config.toml", "127.0.0.1");

    // pull-templates
    wt.run(&["pull", "--full", "--all", "--format", "json"])
        .assert_code(0)
        .assert_json("/report/success", true)
        .assert_json_at_least("/report/pulled", 3);
    wt.assert_file_contains(
        "templates/misc/Template_Infobox_existing.wiki",
        "Existing fixture v1",
    );
    wt.assert_file_contains(
        "templates/misc/_redirects/Template_Infobox_alias.wiki",
        "#REDIRECT",
    );

    // inspect-synced-status
    wt.run(&["status", "--templates", "--format", "json"])
        .assert_code(0)
        .assert_json("/sync_ledger_ready", true)
        .assert_json("/plan/modified_local", 0)
        .assert_json("/plan/changes", json!([]));

    // --- Edit locally and push against a remote that moved on ---

    // modify-existing-template, add-new-template
    wt.install(
        fixture("existing-updated.wiki"),
        "templates/misc/Template_Infobox_existing.wiki",
    );
    wt.install(
        fixture("created.wiki"),
        "templates/misc/Template_Infobox_created.wiki",
    );

    // diff-local-changes
    wt.run(&["diff", "--templates", "--format", "json"])
        .assert_code(0)
        .assert_json_item("/changes", "/title", "Template:Infobox existing")
        .assert_json_item("/changes", "/title", "Template:Infobox created");

    // advance-remote-revision
    wiki.set_page(MediaWikiPage {
        title: "Template:Infobox existing".to_owned(),
        namespace: 10,
        page_id: 10,
        revision_id: 102,
        timestamp: "2026-08-27T11:00:00Z".to_owned(),
        content: "<includeonly>Concurrent remote edit</includeonly>\n".to_owned(),
    });

    // detect-stale-revision
    wt.run(&[
        "push",
        "--all",
        "--templates",
        "--summary",
        "Wikitest preview",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/report/success", false)
    .assert_json_contains("/report/conflicts", "Template:Infobox existing")
    .assert_json_item("/report/pages", "/action", "would_create");

    // preview-force-revision-bound-push
    let push_args = [
        "push",
        "--force",
        "--all",
        "--templates",
        "--summary",
        "Wikitest revision-bound push",
    ];
    let preview = wt.run(&[&push_args[..], &["--format", "json"]].concat());
    preview
        .assert_code(0)
        .assert_json("/mode", "preview")
        .assert_json("/report/success", true);
    let plan_id = preview.json_str("/report/plan_id");

    // apply-force-revision-bound-push
    let apply = wt.run_with_env(
        &[
            &push_args[..],
            &["--apply", &plan_id, "--format", "json", "--progress"],
        ]
        .concat(),
        BOT,
    );
    apply
        .assert_code(0)
        .assert_json("/report/success", true)
        .assert_json("/report/created", 1)
        .assert_json("/report/updated", 1);
    for progress in [
        r#""schema":"wikitool.push-progress.v1""#,
        r#""phase":"applying","completed":1,"total":2"#,
        r#""phase":"processed","completed":2,"total":2"#,
    ] {
        apply.assert_stderr_contains(progress);
    }

    // --- A push whose response is lost is not retried ---

    // modify-ambiguous-template
    wt.install(
        fixture("ambiguous-updated.wiki"),
        "templates/misc/Template_Infobox_ambiguous.wiki",
    );

    // preview-force-ambiguous-one-shot-push
    let push_args = [
        "push",
        "--force",
        "--all",
        "--templates",
        "--summary",
        "Wikitest ambiguous one-shot push",
    ];
    let preview = wt.run(&[&push_args[..], &["--format", "json"]].concat());
    preview
        .assert_code(0)
        .assert_json("/mode", "preview")
        .assert_json("/report/success", true);
    let plan_id = preview.json_str("/report/plan_id");

    // apply-force-ambiguous-one-shot-push
    wt.run_with_env(
        &[&push_args[..], &["--apply", &plan_id, "--format", "json"]].concat(),
        BOT,
    )
    .assert_code(1)
    .assert_json("/report/success", false)
    .assert_json("/report/pushed", 0);

    // --- Standalone delete ---

    // preview-standalone-delete
    let delete_args = [
        "delete",
        "Template:Delete normal",
        "--reason",
        "Wikitest standalone delete",
    ];
    let preview = wt.run(&[&delete_args[..], &["--format", "json"]].concat());
    preview
        .assert_code(0)
        .assert_json("/mode", "plan")
        .assert_json("/plan/canonical_title", "Template:Delete normal")
        .assert_json("/plan/observed_state", "present");
    let plan_id = preview.json_str("/plan/plan_id");

    // apply-standalone-delete
    wt.run_with_env(
        &[&delete_args[..], &["--apply", &plan_id, "--format", "json"]].concat(),
        BOT,
    )
    .assert_code(0)
    .assert_json("/success", true)
    .assert_json("/mode", "apply")
    .assert_json("/remote/status", "deleted")
    .assert_json_at_least("/remote/deletion_log_id", 1);

    // --- Delete of a page that vanished first (missingtitle) ---

    // preview-missingtitle-delete
    let delete_args = [
        "delete",
        "Template:Delete missingtitle",
        "--reason",
        "Wikitest missingtitle delete",
    ];
    let preview = wt.run(&[&delete_args[..], &["--format", "json"]].concat());
    preview
        .assert_code(0)
        .assert_json("/mode", "plan")
        .assert_json("/plan/observed_state", "present");
    let plan_id = preview.json_str("/plan/plan_id");

    // apply-missingtitle-delete
    wt.run_with_env(
        &[&delete_args[..], &["--apply", &plan_id, "--format", "json"]].concat(),
        BOT,
    )
    .assert_code(0)
    .assert_json("/success", true)
    .assert_json("/remote/status", "already_missing");

    // --- Ambiguous delete reconciled from a visible delete-log marker ---

    // preview-ambiguous-reconciled-delete
    let delete_args = [
        "delete",
        "Template:Delete reconciled",
        "--reason",
        "Wikitest ambiguous reconciled delete",
    ];
    let preview = wt.run(&[&delete_args[..], &["--format", "json"]].concat());
    preview.assert_code(0).assert_json("/mode", "plan");
    let plan_id = preview.json_str("/plan/plan_id");

    // apply-ambiguous-reconciled-delete
    let apply = wt.run_with_env(
        &[&delete_args[..], &["--apply", &plan_id, "--format", "json"]].concat(),
        BOT,
    );
    apply
        .assert_code(1)
        .assert_json("/success", false)
        .assert_json("/status", "outcome_ambiguous")
        .assert_json("/title", "Template:Delete reconciled")
        .assert_json("/next_action/operation", "delete");
    let mutation = apply.json_u64("/mutation_id");
    assert!(mutation >= 1, "{apply}");
    let mutation = mutation.to_string();

    // show-ambiguous-delete
    wt.run(&["mutation", "show", "delete", &mutation, "--format", "json"])
        .assert_code(0)
        .assert_json("/operation", "delete")
        .assert_json("/title", "Template:Delete reconciled")
        .assert_json("/phase", "outcome_ambiguous")
        .assert_json("/request_started", true)
        .assert_json("/local_effect_status", "backup_ready");

    // reconcile-ambiguous-delete-from-visible-marker
    wt.run(&[
        "mutation",
        "reconcile",
        "delete",
        &mutation,
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/operation", "delete")
    .assert_json("/title", "Template:Delete reconciled")
    .assert_json("/status", "state_advanced")
    .assert_json("/request_count", 3)
    .assert_json_at_least("/deletion_log_id", 1);

    // show-reconciled-delete
    wt.run(&["mutation", "show", "delete", &mutation, "--format", "json"])
        .assert_code(0)
        .assert_json("/title", "Template:Delete reconciled")
        .assert_json("/phase", "state_advanced")
        .assert_json("/local_effect_status", "complete");

    // --- Ambiguous delete whose marker is hidden: operator closure ---

    // preview-ambiguous-irreconcilable-delete
    let delete_args = [
        "delete",
        "Template:Delete irreconcilable",
        "--reason",
        "Wikitest ambiguous irreconcilable delete",
    ];
    let preview = wt.run(&[&delete_args[..], &["--format", "json"]].concat());
    preview.assert_code(0).assert_json("/mode", "plan");
    let plan_id = preview.json_str("/plan/plan_id");

    // apply-ambiguous-irreconcilable-delete
    let apply = wt.run_with_env(
        &[&delete_args[..], &["--apply", &plan_id, "--format", "json"]].concat(),
        BOT,
    );
    apply
        .assert_code(1)
        .assert_json("/success", false)
        .assert_json("/status", "outcome_ambiguous")
        .assert_json("/title", "Template:Delete irreconcilable");
    let mutation = apply.json_u64("/mutation_id");
    assert!(mutation >= 1, "{apply}");
    let mutation = mutation.to_string();

    // show-irreconcilable-delete
    wt.run(&["mutation", "show", "delete", &mutation, "--format", "json"])
        .assert_code(0)
        .assert_json("/title", "Template:Delete irreconcilable")
        .assert_json("/phase", "outcome_ambiguous")
        .assert_json("/request_started", true);

    // reconcile-irreconcilable-delete
    wt.run(&[
        "mutation",
        "reconcile",
        "delete",
        &mutation,
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/operation", "delete")
    .assert_json("/title", "Template:Delete irreconcilable")
    .assert_json("/status", "still_ambiguous")
    .assert_json("/request_count", 3);

    // close-irreconcilable-delete
    let close = wt.run(&[
        "mutation",
        "close",
        "delete",
        &mutation,
        "--actor",
        "Wikitest operator",
        "--reason",
        "Fixture hides the delete marker",
        "--confirm",
        "--format",
        "json",
    ]);
    close
        .assert_code(0)
        .assert_json("/operation", "delete")
        .assert_json("/title", "Template:Delete irreconcilable")
        .assert_json("/previous_phase", "outcome_ambiguous")
        .assert_json("/terminal_outcome", "operator_closed_unresolved")
        .assert_json("/actor", "Wikitest operator")
        .assert_json("/reason", "Fixture hides the delete marker")
        .assert_json("/sync_baseline_invalidated", true);
    close.json_str("/next_action");

    // show-closed-delete
    wt.run(&["mutation", "show", "delete", &mutation, "--format", "json"])
        .assert_code(0)
        .assert_json("/title", "Template:Delete irreconcilable")
        .assert_json("/phase", "resolved")
        .assert_json("/terminal_outcome", "operator_closed_unresolved")
        .assert_json("/closure/actor", "Wikitest operator")
        .assert_json("/closure/reason", "Fixture hides the delete marker");

    // list-closed-delete
    wt.run(&["mutation", "list", "--all", "--format", "json"])
        .assert_code(0)
        .assert_json("/unresolved_only", false)
        .assert_json_item("/mutations", "/title", "Template:Delete irreconcilable")
        .assert_json_item(
            "/mutations",
            "/terminal_outcome",
            "operator_closed_unresolved",
        );

    // reject-write-before-refresh
    wt.run(&[
        "delete",
        "Template:Delete irreconcilable",
        "--reason",
        "Wikitest blocked before refresh",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_stderr_contains("was invalidated by operator closure")
    .assert_stderr_contains("target-bound pull");

    // --- Refresh, then delete the recreated page ---

    // recreate-closed-delete-target
    wiki.set_page(MediaWikiPage {
        title: "Template:Delete irreconcilable".to_owned(),
        namespace: 10,
        page_id: 23,
        revision_id: 900,
        timestamp: "2026-08-27T23:00:00Z".to_owned(),
        content: "<includeonly>Irreconcilable delete fixture</includeonly>\n".to_owned(),
    });

    // refresh-after-operator-closure
    wt.run(&["pull", "--full", "--all", "--format", "json"])
        .assert_code(0)
        .assert_json("/report/success", true)
        .assert_json_item("/report/pages", "/title", "Template:Delete irreconcilable");

    // preview-post-pull-delete
    let delete_args = [
        "delete",
        "Template:Delete irreconcilable",
        "--reason",
        "Wikitest post-pull delete",
    ];
    let preview = wt.run(&[&delete_args[..], &["--format", "json"]].concat());
    preview
        .assert_code(0)
        .assert_json("/mode", "plan")
        .assert_json("/plan/observed_revision_id", 900);
    let plan_id = preview.json_str("/plan/plan_id");

    // apply-post-pull-delete
    wt.run_with_env(
        &[&delete_args[..], &["--apply", &plan_id, "--format", "json"]].concat(),
        BOT,
    )
    .assert_code(0)
    .assert_json("/success", true)
    .assert_json("/remote/status", "deleted")
    .assert_json("/remote/observed_revision_id", 900);

    // --- assert-authenticated-one-shot-wire-contract ---

    // Every write is authenticated and sent once; ambiguous writes are never retried.
    wiki.assert_requests("POST", &[("action", "edit")], 3);
    wiki.assert_requests(
        "POST",
        &[("action", "login"), ("lgname", "wikitest-bot")],
        7,
    );
    wiki.assert_requests(
        "POST",
        &[
            ("action", "edit"),
            ("title", "Template:Infobox existing"),
            ("baserevid", "102"),
        ],
        1,
    );
    wiki.assert_requests(
        "POST",
        &[
            ("action", "edit"),
            ("title", "Template:Infobox ambiguous"),
            ("baserevid", "301"),
        ],
        1,
    );
    wiki.assert_requests(
        "POST",
        &[
            ("action", "edit"),
            ("title", "Template:Infobox created"),
            ("createonly", "1"),
        ],
        1,
    );
    wiki.assert_requests("POST", &[("action", "delete")], 5);
    for (title, count) in [
        ("Template:Delete normal", 1),
        ("Template:Delete missingtitle", 1),
        ("Template:Delete reconciled", 1),
        ("Template:Delete irreconcilable", 2),
    ] {
        wiki.assert_requests("POST", &[("action", "delete"), ("title", title)], count);
    }

    // Reconciliation reads the full, paginated delete-log lineage of each ambiguous title.
    let logevents = [("action", "query"), ("list", "logevents")];
    wiki.assert_requests("GET", &logevents, 4);
    wiki.assert_requests("GET", &[&logevents[..], &[("lecontinue", "1")]].concat(), 2);
    for title in [
        "Template:Delete reconciled",
        "Template:Delete irreconcilable",
    ] {
        wiki.assert_requests("GET", &[&logevents[..], &[("letitle", title)]].concat(), 2);
        wiki.assert_requests(
            "GET",
            &[&logevents[..], &[("letitle", title), ("lecontinue", "1")]].concat(),
            1,
        );
    }

    // The remote holds exactly the pushed bytes, including the push whose response was lost.
    for (title, file) in [
        ("Template:Infobox existing", "existing-updated.wiki"),
        ("Template:Infobox created", "created.wiki"),
        ("Template:Infobox ambiguous", "ambiguous-updated.wiki"),
    ] {
        let expected = fs::read_to_string(fixture(file)).expect("read fixture");
        assert_eq!(
            wiki.page(title).map(|page| page.content),
            Some(expected),
            "remote content of {title}"
        );
    }
    for title in [
        "Template:Delete normal",
        "Template:Delete missingtitle",
        "Template:Delete reconciled",
        "Template:Delete irreconcilable",
    ] {
        assert_eq!(wiki.page(title), None, "{title} should be deleted");
    }

    // Each delete left one log entry with the planned reason, except the hidden marker.
    let visible_with = |title: &str, reason: &str| {
        wiki.delete_logs(title)
            .iter()
            .filter(|log| {
                !log.comment_hidden
                    && log
                        .comment
                        .as_deref()
                        .is_some_and(|comment| comment.contains(reason))
            })
            .count()
    };
    let logs_of = |title: &str| wiki.delete_logs(title);
    assert_eq!(
        visible_with("Template:Delete normal", "Wikitest standalone delete"),
        1,
        "{:#?}",
        logs_of("Template:Delete normal")
    );
    assert_eq!(
        logs_of("Template:Delete missingtitle").len(),
        0,
        "{:#?}",
        logs_of("Template:Delete missingtitle")
    );
    assert_eq!(
        visible_with(
            "Template:Delete reconciled",
            "Wikitest ambiguous reconciled delete"
        ),
        1,
        "{:#?}",
        logs_of("Template:Delete reconciled")
    );
    let irreconcilable = logs_of("Template:Delete irreconcilable");
    assert_eq!(
        irreconcilable
            .iter()
            .filter(|log| log.comment_hidden)
            .count(),
        1,
        "{irreconcilable:#?}"
    );
    assert_eq!(
        visible_with(
            "Template:Delete irreconcilable",
            "Wikitest post-pull delete"
        ),
        1,
        "{irreconcilable:#?}"
    );
}
