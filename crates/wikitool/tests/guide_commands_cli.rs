//! Procedures and command discovery from the binary: `guide` prints the embedded skills and
//! the selected site adapter's guidance byte for byte, and `commands` finds commands by
//! purpose. Neither needs an initialized project.

use std::fs;
use std::path::Path;

use wikitest::harness::Project;

fn canonical_skill(relative: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.agents/skills")
            .join(relative),
    )
    .expect("canonical skill source")
}

#[test]
fn guide_prints_orientation_and_exact_skill_bytes_without_a_project() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    wt.run(&["guide"])
        .assert_code(0)
        .assert_stdout_contains("  wikitool      Use Wikitool for MediaWiki retrieval")
        .assert_stdout_contains("  wiki-writing  ")
        .assert_stdout_contains("  prose-review  ")
        .assert_stdout_contains("Sync baseline:")
        .assert_stdout_contains("Acceptance:")
        .assert_stdout_contains("  wikitool/sync-and-acceptance  ")
        .assert_stdout_contains("Site adapter mediawiki-generic: no supplemental guidance");

    let skill = wt.run(&["guide", "wikitool"]);
    skill.assert_code(0);
    assert_eq!(skill.stdout, canonical_skill("wikitool/SKILL.md"));

    let reference = wt.run(&["guide", "sync-and-acceptance"]);
    reference.assert_code(0);
    assert_eq!(
        reference.stdout,
        canonical_skill("wikitool/references/sync-and-acceptance.md")
    );

    wt.run(&["guide", "frozen-packet"])
        .assert_code(1)
        .assert_stderr_contains("prose-review/frozen-packet")
        .assert_stderr_contains("wiki-writing/frozen-packet");
    wt.run(&["guide", "adapter:editorial"])
        .assert_code(1)
        .assert_stderr_contains("declares no guidance documents");
}

#[test]
fn guide_lists_and_prints_the_selected_adapters_guidance() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let generic = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site_adapters/generic/site-adapter.toml"),
    )
    .expect("generic adapter")
    .replace(
        "adapter_id = \"mediawiki-generic\"",
        "adapter_id = \"test-wiki\"",
    )
    .replace(
        "guidance_documents = []",
        "guidance_documents = [\"editorial.md\", \"notes/style.md\"]",
    );
    fs::create_dir_all(wt.path().join("site-adapter/notes")).expect("adapter dir");
    fs::write(wt.path().join("site-adapter/site-adapter.toml"), generic).expect("adapter");
    fs::write(
        wt.path().join("site-adapter/editorial.md"),
        "# Editorial\r\n\r\nExact bytes, CRLF kept.\r\n",
    )
    .expect("editorial guidance");
    fs::write(wt.path().join("site-adapter/notes/style.md"), "# Style\n").expect("style");
    wt.run(&[
        "init",
        "--no-network",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);

    wt.run(&["guide"])
        .assert_code(0)
        .assert_stdout_contains("Site adapter test-wiki guidance")
        .assert_stdout_contains("adapter:editorial")
        .assert_stdout_contains("site-adapter/editorial.md")
        .assert_stdout_contains("adapter:notes/style");

    let editorial = wt.run(&["guide", "adapter:editorial"]);
    editorial.assert_code(0);
    assert_eq!(
        editorial.stdout,
        "# Editorial\r\n\r\nExact bytes, CRLF kept.\r\n"
    );
    wt.run(&["guide", "adapter:missing"])
        .assert_code(1)
        .assert_stderr_contains("available: editorial, notes/style");
}

#[test]
fn commands_find_commands_by_purpose_with_audience_tags() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    wt.run(&["commands", "page", "exists"])
        .assert_code(0)
        .assert_stdout_contains("source wiki-search [agent]")
        .assert_stdout_contains("  Search the configured wiki API");
    wt.run(&["commands", "reset", "database"])
        .assert_code(0)
        .assert_stdout_contains("db reset [operator]")
        .assert_stdout_contains("1 of 1 matching commands");
    wt.run(&["commands", "no-such-purpose-anywhere"])
        .assert_code(0)
        .assert_stdout_contains("No command matches every word");

    let listing = wt.run(&["commands", "--limit", "3", "--format", "json"]);
    listing
        .assert_code(0)
        .assert_json("/schema_version", "wikitool_commands_v1")
        .assert_json("/shown_count", 3)
        .assert_json_at_least("/matching_count", 90);
    let lint = wt.run(&["commands", "lint", "article", "--format", "json"]);
    lint.assert_code(0)
        .assert_json_item("/commands", "/path", "article lint");
}
