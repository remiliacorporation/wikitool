use serde_json::Value;
use std::fs;
use wikitest::harness::Project;

fn plan(wt: &Project) -> Value {
    wt.run(&[
        "templates",
        "migration-plan",
        "migration.json",
        "--format",
        "json",
    ])
    .assert_code(0)
    .json()
}

#[test]
fn plan_binds_full_current_inventory_without_catalog_or_rewrites() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let root = wt.path();
    fs::create_dir_all(root.join(".wikitool")).unwrap();
    fs::create_dir_all(root.join("wiki_content/Main")).unwrap();
    fs::write(
        root.join("migration.json"),
        r#"{
        "schema":"template_migration_spec_v1", "from_template":"Old", "to_template":"New",
        "title_case":"first_letter", "parameter_renames":{"old":"new"}
    }"#,
    )
    .unwrap();
    let source = "Start {{Old | old = value}} end";
    let article = root.join("wiki_content/Main/Article.wiki");
    fs::write(&article, source).unwrap();
    fs::write(root.join("wiki_content/Main/Unrelated.wiki"), "Other").unwrap();
    let first = plan(&wt);
    assert_eq!(first["scanned_files"], 2);
    assert_eq!(first["affected_files"], 1);
    assert_eq!(first["mechanical_patch_count"], 2);
    assert_eq!(first["retirement_ready"], false);
    assert_eq!(first["files"][0]["invocations"][0]["start_byte"], 6);
    assert_eq!(fs::read_to_string(&article).unwrap(), source);
    assert_eq!(first["plan_id"], plan(&wt)["plan_id"]);
    fs::write(root.join("wiki_content/Main/Unrelated.wiki"), "Changed").unwrap();
    assert_ne!(first["plan_id"], plan(&wt)["plan_id"]);
    assert!(!root.join(".wikitool/sync/sync.sqlite3").exists());
    assert!(!root.join(".wikitool/data/wikitool.db").exists());
}
