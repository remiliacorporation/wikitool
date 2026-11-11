//! Profiled HTML-to-wikitext compilation: captured HTML and ordered media evidence compile
//! through independent source and target profiles, source-authored content is preserved,
//! unadmitted structures and CSS/JavaScript evidence are reported, a stale capture receipt is
//! refused, and retained video projects to a native template.
//!
//! Ported from `wikitest/scenarios/mechanical/html-to-wikitext/scenario.json`; comments name the
//! scenario's step IDs. One test, because the stale-receipt refusal and the video compilation
//! reuse the profiles and evidence installed for the first compilation.

use std::path::PathBuf;

use wikitest::harness::Project;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../wikitest/scenarios/mechanical/html-to-wikitext/fixtures")
        .join(name)
}

const COMPILED: &str = ".wikitool/imports/Fixture_subject.wiki";
const VIDEO: &str = ".wikitool/imports/Video.wiki";

/// The `import html-to-wikitext` invocation every step shares, apart from its inputs.
fn compile<'a>(
    html: &'a str,
    capture_receipt: &'a str,
    target_profile: &'a str,
    media_inventory: &'a str,
    output: &'a str,
) -> Vec<&'a str> {
    vec![
        "import",
        "html-to-wikitext",
        html,
        "--source-profile",
        "profiles/source.json",
        "--capture-receipt",
        capture_receipt,
        "--target-profile",
        target_profile,
        "--canonical-title",
        "Fixture subject",
        "--canonical-url",
        "https://fixture.example/wiki/Fixture_subject",
        "--source-key",
        "fixture",
        "--media-scope",
        "fixture",
        "--media-inventory",
        media_inventory,
        "--output",
        output,
        "--format",
        "json",
    ]
}

#[test]
fn profiled_compilation_stale_receipt_and_native_video() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    // --- Compile captured HTML through source and target profiles ---

    // install-source-profile, install-target-profile, install-html, install-capture-receipt,
    // install-stale-capture-receipt, install-media-inventory
    wt.install(fixture("source-profile.json"), "profiles/source.json");
    wt.install(fixture("target-profile.json"), "profiles/target.json");
    wt.install(fixture("page.html"), "evidence/page.html");
    wt.install(
        fixture("capture-receipt.json"),
        "evidence/capture-receipt.json",
    );
    wt.install(
        fixture("stale-capture-receipt.json"),
        "evidence/stale-capture-receipt.json",
    );
    wt.install(
        fixture("media-inventory.json"),
        "evidence/media-inventory.json",
    );

    // compile-profiled-html
    let compiled = wt.run(&compile(
        "evidence/page.html",
        "evidence/capture-receipt.json",
        "profiles/target.json",
        "evidence/media-inventory.json",
        COMPILED,
    ));
    compiled
        .assert_code(0)
        .assert_json("/schema", "wikitool.import-html-to-wikitext.v2")
        .assert_json("/status", "compiled")
        .assert_json("/source_profile_id", "fixture-rendered-html-v1")
        .assert_json("/target_profile_id", "fixture-target-v1")
        .assert_json(
            "/source_html_sha256",
            "a5f75f416ac3df245febb9cdda6557f23ac361d61ed2f66aa9fef3f2d554c4a5",
        )
        .assert_json(
            "/capture_receipt_sha256",
            "43818ed735980d73b65322e8697c313d1c12c9bf805048a2dea073f023207672",
        )
        .assert_json("/representation", "rendered_dom")
        .assert_json(
            "/capture_final_url",
            "https://fixture.example/wiki/Fixture_subject",
        )
        .assert_json("/capture_javascript_executed", true)
        .assert_json(
            "/media_inventory_sha256",
            "eb1fdceb82494ece20ef17880305e5b172cc28328bc945b00f1f78e40d433dd1",
        )
        .assert_json(
            "/wikitext_sha256",
            "ba8e9c15d07ae40093e2a4cf50f40a0374504d7836589e40600c634185055e07",
        )
        .assert_json("/coverage/native_infoboxes", 1)
        .assert_json("/coverage/archived_audio_elements", 1)
        .assert_json("/coverage/discarded_style_elements", 1)
        .assert_json("/coverage/discarded_script_elements", 1)
        .assert_json("/coverage/discarded_hidden_elements", 3)
        .assert_json("/coverage/discarded_profiled_elements", 1)
        .assert_json("/coverage/discarded_interaction_elements", 1)
        .assert_json("/media_occurrences_consumed", 3)
        .assert_json_contains(
            "/used_media",
            "https://fixture.example/media/Source_Capture.ogg",
        )
        .assert_json_item("/unmapped_structures", "/classes/0", "mystery-box");
    for (item_pointer, value) in [
        ("/kind", "external_stylesheet"),
        ("/kind", "inline_style"),
        ("/kind", "external_script"),
        ("/kind", "inline_script"),
        (
            "/locator/resolved_url",
            "https://fixture.example/assets/site.css",
        ),
        (
            "/locator/resolved_url",
            "https://fixture.example/assets/application.js",
        ),
        ("/classification", "declarative"),
        ("/disposition", "not_applied"),
        ("/disposition", "not_executed"),
        ("/disposition", "declarative_evidence_only"),
    ] {
        compiled.assert_json_item("/resource_observations", item_pointer, value);
    }
    for needle in [
        "{{Subject box",
        "| below = Source-authored context",
        "{{Notice box",
        "| image_content = {{Archive image",
        "|filename=Notice.png",
        "|label=Source Capture",
        "|source1_filename=Source_Capture.ogg",
    ] {
        wt.assert_file_contains(COMPILED, needle);
    }
    for needle in ["Audio sample", "Coverage", "| type = Video game"] {
        wt.assert_file_lacks(COMPILED, needle);
    }

    // reject-stale-capture-receipt
    wt.run(&compile(
        "evidence/page.html",
        "evidence/stale-capture-receipt.json",
        "profiles/target.json",
        "evidence/media-inventory.json",
        ".wikitool/imports/Stale_Fixture_subject.wiki",
    ))
    .assert_code(1)
    .assert_stderr_contains("HTML capture receipt html_sha256 does not match supplied HTML");
    wt.assert_missing(".wikitool/imports/Stale_Fixture_subject.wiki");

    // --- Retained video projects to the native template ---

    // install-video-target-profile-json, install-video-html, install-video-capture-receipt-json,
    // install-video-media-inventory-json
    wt.install(
        fixture("video-target-profile.json"),
        "profiles/video-target.json",
    );
    wt.install(fixture("video.html"), "evidence/video.html");
    wt.install(
        fixture("video-capture-receipt.json"),
        "evidence/video-capture.json",
    );
    wt.install(
        fixture("video-media-inventory.json"),
        "evidence/video-media.json",
    );

    // compile-retained-video
    wt.run(&compile(
        "evidence/video.html",
        "evidence/video-capture.json",
        "profiles/video-target.json",
        "evidence/video-media.json",
        VIDEO,
    ))
    .assert_code(0)
    .assert_json("/status", "compiled")
    .assert_json("/coverage/archived_video_elements", 1)
    .assert_json("/coverage/archived_video_locators", 3)
    .assert_json("/media_occurrences_consumed", 3);
    for needle in [
        "{{Archive video|site=fixture|label=Unused scene",
        "|width=320|height=240|loop=1|muted=1|poster_sha256=",
        "|source1_type=mp4|source1_filename=Scene.mp4",
        "|source2_type=webm|source2_filename=Scene.webm",
        "Original scene description.",
    ] {
        wt.assert_file_contains(VIDEO, needle);
    }
}
