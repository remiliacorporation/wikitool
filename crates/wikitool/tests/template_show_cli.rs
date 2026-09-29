use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn wikitool(root: &Path, args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_wikitool"))
        .arg("--project-root")
        .arg(root)
        .args(args)
        .output()
        .expect("run wikitool");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn template_show_brief_exposes_all_declared_keys_without_full_text_detail() {
    let root = tempfile::tempdir().expect("project fixture");
    let templates = root.path().join("templates/misc");
    fs::create_dir_all(&templates).expect("template directory");
    fs::write(
        templates.join("Template_Test.wiki"),
        "<includeonly>{{{alpha|}}} {{{beta|}}} {{{gamma|}}} {{{delta|}}} {{{epsilon|}}} {{{zeta|}}} {{{eta|}}} {{{theta|}}} {{{iota|}}} {{{kappa|}}} {{{lambda|}}} {{{mu|}}} {{{nu|}}}</includeonly><noinclude><templatedata>{\"params\":{\"templatedata_only\":{\"suggested\":true}}}</templatedata></noinclude>",
    )
    .expect("template source");
    wikitool(root.path(), &["templates", "catalog", "build"]);

    let json = wikitool(
        root.path(),
        &[
            "templates",
            "show",
            "Test",
            "--format",
            "json",
            "--view",
            "brief",
        ],
    );
    let brief: serde_json::Value = serde_json::from_slice(&json.stdout).expect("brief JSON");
    let mut declared_keys = brief["contract"]["declared_parameter_keys"]
        .as_array()
        .expect("all declared names")
        .iter()
        .map(|value| value.as_str().expect("parameter name"))
        .collect::<Vec<_>>();
    declared_keys.sort_unstable();
    assert_eq!(
        declared_keys,
        [
            "alpha",
            "beta",
            "delta",
            "epsilon",
            "eta",
            "gamma",
            "iota",
            "kappa",
            "lambda",
            "mu",
            "nu",
            "templatedata_only",
            "theta",
            "zeta"
        ]
    );
    assert_eq!(brief["contract"]["declared_parameter_count"], 14);

    let text = wikitool(
        root.path(),
        &["templates", "show", "Test", "--view", "brief"],
    );
    let text = String::from_utf8(text.stdout).expect("brief text");
    assert!(text.contains("view: brief\n"));
    assert!(text.contains("declared_parameter_count: 14\n"));
    assert!(text.contains("declared_parameter_keys_more: 2\n"));
    assert!(
        !text.contains("parameter: alpha ("),
        "brief expanded full detail"
    );

    let full = wikitool(
        root.path(),
        &["templates", "show", "Test", "--view", "full"],
    );
    let full = String::from_utf8(full.stdout).expect("full text");
    assert!(full.contains("parameter: alpha ("));
}
