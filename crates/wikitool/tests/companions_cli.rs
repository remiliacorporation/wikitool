//! Optional release companions: Wikitool stays usable without a companion manifest, and
//! diagnoses a present, separately owned Papertiger pack without initializing its task authority.
//!
//! Ported from `wikitest/scenarios/mechanical/optional-companions/scenario.json`; comments name
//! the scenario's step IDs. The companion "binaries" are short text fixtures that are only
//! inspected, never executed. One test, because the second inspection runs in the project the
//! first one found empty.

use std::path::PathBuf;

use serde_json::json;
use wikitest::harness::Project;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/optional-companions/fixtures")
        .join(name)
}

#[test]
fn companion_manifest_absent_then_present() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    // inspect-absent
    wt.run(&[
        "companions",
        "--manifest",
        "missing/release-companions.json",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/schema", "wikitool.companion-diagnostics.v1")
    .assert_json("/status", "optional_manifest_absent")
    .assert_json("/wikitool_available", true)
    .assert_json("/companions", json!([]));

    // install-companion-manifest, install-planner, install-mise, install-upstream-manifest,
    // install-agent-contract
    wt.install(
        fixture("release-companions.json"),
        "bundle/release-companions.json",
    );
    wt.install(
        fixture("papertiger.exe"),
        "bundle/papertiger/papertiger.exe",
    );
    wt.install(
        fixture("papertiger-mise.exe"),
        "bundle/papertiger/papertiger-mise.exe",
    );
    wt.install(fixture("manifest.json"), "bundle/papertiger/manifest.json");
    wt.install(
        fixture("agent_integration.md"),
        "bundle/papertiger/agent_integration.md",
    );

    // inspect-present
    wt.run(&[
        "companions",
        "--manifest",
        "bundle/release-companions.json",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/status", "ok")
    .assert_json("/wikitool_available", true)
    .assert_json_item("/companions", "/id", "papertiger")
    .assert_json_item("/companions", "/version", "0.9.0")
    .assert_json_item(
        "/companions",
        "/source_commit",
        "3f2a1ef6f40ad01ca9b07d44b28b10d7a3276af0",
    )
    .assert_json_item("/companions", "/required_for_wikitool", false)
    .assert_json_item("/companions", "/project_lifecycle_owner", "papertiger")
    .assert_json_item("/companions", "/status", "available");
}
