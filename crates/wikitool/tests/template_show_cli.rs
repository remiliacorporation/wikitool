use std::fs;
use wikitest::harness::Project;

#[test]
fn template_show_brief_exposes_all_declared_keys_without_full_text_detail() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let templates = wt.path().join("templates/misc");
    fs::create_dir_all(&templates).expect("template directory");
    fs::write(
        templates.join("Template_Test.wiki"),
        "<includeonly>{{{alpha|}}} {{{beta|}}} {{{gamma|}}} {{{delta|}}} {{{epsilon|}}} {{{zeta|}}} {{{eta|}}} {{{theta|}}} {{{iota|}}} {{{kappa|}}} {{{lambda|}}} {{{mu|}}} {{{nu|}}}</includeonly><noinclude><templatedata>{\"params\":{\"templatedata_only\":{\"suggested\":true}}}</templatedata></noinclude>",
    )
    .expect("template source");
    wt.run(&["templates", "catalog", "build"]).assert_code(0);

    let brief = wt
        .run(&[
            "templates",
            "show",
            "Test",
            "--format",
            "json",
            "--view",
            "brief",
        ])
        .assert_code(0)
        .json();
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

    let text = wt.run(&["templates", "show", "Test", "--view", "brief"]);
    let text = &text.assert_code(0).stdout;
    assert!(text.contains("view: brief\n"));
    assert!(text.contains("declared_parameter_count: 14\n"));
    assert!(text.contains("declared_parameter_keys_more: 2\n"));
    assert!(
        !text.contains("parameter: alpha ("),
        "brief expanded full detail"
    );

    let full = wt.run(&["templates", "show", "Test", "--view", "full"]);
    let full = &full.assert_code(0).stdout;
    assert!(full.contains("parameter: alpha ("));
}
