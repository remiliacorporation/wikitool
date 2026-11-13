//! `wikitool ontology export` vendors an ontology checkout into the project and generates
//! `Module:Ontology/data`; `ontology generate --check` fails when the module no longer matches
//! the vendored copy. The ontology is a small git checkout built in a temporary directory whose
//! path contains spaces.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use tempfile::TempDir;
use wikitest::harness::Project;

const EXPORT: &str = "vendor/ontology/ontology.json";
const MODULE: &str = "templates/ontology/Module_Ontology/data.lua";

struct Ontology {
    temp: TempDir,
    dir: PathBuf,
}

impl Ontology {
    fn new() -> Self {
        let temp = tempfile::Builder::new()
            .prefix("ontology checkout ")
            .tempdir()
            .expect("tempdir");
        let dir = temp.path().join("ontology");
        fs::write(temp.path().join("empty.gitconfig"), "").expect("gitconfig");
        let ontology = Self { temp, dir };
        let files = [
            (
                "registry/agent.jsonld",
                json!({ "@id": "remilia:Agent", "@type": "OntologyClass", "label": "Agent" })
                    .to_string(),
            ),
            (
                "mappings.sssom.tsv",
                "subject_id\tpredicate_id\tobject_id\n\
                 remilia:Agent\tskos:exactMatch\tprov:Agent\n"
                    .to_string(),
            ),
            (
                "renames.sssom.tsv",
                "subject_id\tpredicate_id\tobject_id\n\
                 rn:Agent\tdcterms:isReplacedBy\tremilia:Agent\n\
                 remilia:Actor\tdcterms:isReplacedBy\tremilia:Agent\n"
                    .to_string(),
            ),
            (
                "v1/imports/product-uses.tsv",
                "# curie_map:\n#   wd: http://www.wikidata.org/entity/\n\
                 product\tiri\tpin\n\
                 test-wiki\twd:Q5\tterms.nt\n"
                    .to_string(),
            ),
            (
                "v1/imports/terms.nt",
                "<http://www.wikidata.org/entity/Q5> <http://www.w3.org/2000/01/rdf-schema#label> \"human\"@en .\n"
                    .to_string(),
            ),
        ];
        for (relative, text) in files {
            let path = ontology.dir.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
            fs::write(path, text).expect("write fixture");
        }
        ontology.git(&["init", "-q"]);
        ontology.git(&["config", "user.name", "Ontology Fixture"]);
        ontology.git(&["config", "user.email", "fixture@example.invalid"]);
        ontology.git(&["config", "core.autocrlf", "false"]);
        ontology.git(&["config", "commit.gpgsign", "false"]);
        ontology.git(&["add", "-A"]);
        ontology.git(&["commit", "-q", "-m", "fixture"]);
        ontology
    }

    /// Runs git in the checkout, isolated from the developer's global and system configuration.
    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.dir)
            .args(args)
            .env(
                "GIT_CONFIG_GLOBAL",
                self.temp.path().join("empty.gitconfig"),
            )
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn path(&self) -> &str {
        self.dir.to_str().expect("UTF-8 path")
    }
}

fn read_json(project: &Project<'_>, relative: &str) -> Value {
    serde_json::from_str(&project.read(relative)).expect("JSON file")
}

#[test]
fn export_then_check_and_an_edited_copy_fails_the_check() {
    let ontology = Ontology::new();
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    wt.run(&[
        "ontology",
        "export",
        "--from",
        ontology.path(),
        "--product",
        "test-wiki",
        "--imports",
        "v1/imports",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/schema_version", "ontology_export_v1")
    .assert_json("/commit", ontology.git(&["rev-parse", "HEAD"]))
    .assert_json("/product", "test-wiki")
    .assert_json("/export_path", EXPORT)
    .assert_json("/module_path", MODULE);
    let export = read_json(&wt, EXPORT);
    assert_eq!(export["external"]["wd:Q5"]["label"], "human");
    // Without an adapter nothing is excluded from the renames.
    assert_eq!(export["renames"]["rn:Agent"], "remilia:Agent");
    wt.assert_file_contains(MODULE, "[\"remilia:Agent\"] = {");

    wt.run(&["ontology", "generate", "--check", "--format", "json"])
        .assert_code(0)
        .assert_json("/schema_version", "ontology_generate_v1")
        .assert_json("/check", true)
        .assert_json("/up_to_date", true)
        .assert_json("/written", false);

    // Hand-edit the vendored copy: the module no longer matches it.
    let module_before = wt.read(MODULE);
    let edited = wt.read(EXPORT).replace("\"Agent\"", "\"Edited agent\"");
    fs::write(wt.path().join(EXPORT), edited).expect("edit vendored copy");
    wt.run(&["ontology", "generate", "--check", "--format", "json"])
        .assert_code(1)
        .assert_json("/up_to_date", false)
        .assert_json("/written", false)
        .assert_stderr_contains("is out of date; run `wikitool ontology generate`");
    assert_eq!(wt.read(MODULE), module_before, "--check must not write");

    wt.run(&["ontology", "generate", "--format", "json"])
        .assert_code(0)
        .assert_json("/up_to_date", false)
        .assert_json("/written", true);
    wt.assert_file_contains(MODULE, "\"Edited agent\"");
    wt.run(&["ontology", "generate", "--check"]).assert_code(0);
}

#[test]
fn export_names_both_ways_to_supply_a_missing_product_or_imports() {
    let ontology = Ontology::new();
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    wt.run(&["ontology", "export", "--from", ontology.path()])
        .assert_code(1)
        .assert_stderr_contains(
            "pass --product, or name the ontology in the site adapter's [ontology]",
        );
    wt.run(&[
        "ontology",
        "export",
        "--from",
        ontology.path(),
        "--product",
        "test-wiki",
    ])
    .assert_code(1)
    .assert_stderr_contains("pass --imports, or set imports in the site adapter's [ontology]");
    wt.assert_missing(EXPORT);
    wt.assert_missing(MODULE);
}

#[test]
fn the_site_adapter_supplies_the_export_settings() {
    let ontology = Ontology::new();
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let generic = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site_adapters/generic/site-adapter.toml"),
    )
    .expect("generic adapter");
    let adapter = format!(
        "{generic}\n[ontology]\n\
         product = \"test-wiki\"\n\
         source = \"https://ontology.example/\"\n\
         local_prefix = \"wiki\"\n\
         local_namespace = \"https://wiki.example/ns/\"\n\
         imports = \"v1/imports\"\n\
         exclude_rename_prefixes = [\"rn\"]\n"
    );
    fs::create_dir_all(wt.path().join("site-adapter")).expect("adapter dir");
    fs::write(wt.path().join("site-adapter/site-adapter.toml"), adapter).expect("adapter");
    wt.run(&[
        "init",
        "--no-network",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);

    wt.run(&[
        "ontology",
        "export",
        "--from",
        ontology.path(),
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/product", "test-wiki");
    let export = read_json(&wt, EXPORT);
    assert_eq!(
        export["renames"],
        json!({ "remilia:Actor": "remilia:Agent" })
    );
    assert_eq!(
        export["source"]["local"],
        json!({ "wiki": "https://wiki.example/ns/" })
    );
    assert_eq!(export["source"]["ontology"], "https://ontology.example/");
}

/// `[ontology] module_sections` narrows the generated module to the sections a wiki's modules
/// read, plus `source`; the vendored copy stays complete and omitting the key restores the
/// whole module byte for byte.
#[test]
fn module_sections_narrow_only_the_generated_module() {
    let ontology = Ontology::new();
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let generic = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site_adapters/generic/site-adapter.toml"),
    )
    .expect("generic adapter");
    let write_adapter = |extra: &str| {
        let adapter = format!(
            "{generic}\n[ontology]\n\
             product = \"test-wiki\"\n\
             source = \"https://ontology.example/\"\n\
             local_prefix = \"wiki\"\n\
             local_namespace = \"https://wiki.example/ns/\"\n\
             imports = \"v1/imports\"\n\
             {extra}"
        );
        fs::create_dir_all(wt.path().join("site-adapter")).expect("adapter dir");
        fs::write(wt.path().join("site-adapter/site-adapter.toml"), adapter).expect("adapter");
    };
    write_adapter("");
    wt.run(&[
        "init",
        "--no-network",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);
    wt.run(&["ontology", "export", "--from", ontology.path()])
        .assert_code(0);
    let full_module = wt.read(MODULE);
    let all_sections = json!([
        "classes",
        "external",
        "families",
        "mappings",
        "properties",
        "relations",
        "renames",
        "schemes",
        "source"
    ]);
    wt.run(&["ontology", "generate", "--check", "--format", "json"])
        .assert_code(0)
        .assert_json("/sections", all_sections.clone());
    assert!(full_module.contains("[\"mappings\"] = {"));

    // Narrowed: the current module no longer matches, and generate drops exactly the others.
    write_adapter("module_sections = [\"classes\", \"schemes\"]\n");
    wt.run(&["ontology", "generate", "--check"]).assert_code(1);
    assert_eq!(wt.read(MODULE), full_module, "--check must not write");
    wt.run(&["ontology", "generate", "--format", "json"])
        .assert_code(0)
        .assert_json("/written", true)
        .assert_json("/sections", json!(["classes", "schemes", "source"]));
    let narrowed = wt.read(MODULE);
    for kept in ["[\"classes\"] = {", "[\"schemes\"] = {", "[\"source\"] = {"] {
        assert!(narrowed.contains(kept), "{kept} missing");
    }
    for dropped in [
        "[\"families\"]",
        "[\"properties\"]",
        "[\"relations\"]",
        "[\"mappings\"]",
        "[\"renames\"]",
        "[\"external\"]",
    ] {
        assert!(!narrowed.contains(dropped), "{dropped} kept");
    }
    wt.run(&["ontology", "generate", "--check"]).assert_code(0);
    // The vendored copy is complete, and export writes the narrowed module too.
    wt.run(&["ontology", "export", "--from", ontology.path()])
        .assert_code(0);
    assert_eq!(
        read_json(&wt, EXPORT)["mappings"]["remilia:Agent"][0]["object"],
        "prov:Agent"
    );
    assert_eq!(wt.read(MODULE), narrowed);

    // A changed list makes the narrowed module out of date.
    write_adapter("module_sections = [\"classes\"]\n");
    wt.run(&["ontology", "generate", "--check", "--format", "json"])
        .assert_code(1)
        .assert_json("/up_to_date", false)
        .assert_json("/sections", json!(["classes", "source"]));

    // Without the key the module is the whole export again, byte for byte.
    write_adapter("");
    wt.run(&["ontology", "generate"]).assert_code(0);
    assert_eq!(wt.read(MODULE), full_module);

    // An unknown section is an adapter error that names the valid ones.
    write_adapter("module_sections = [\"classes\", \"mapping\"]\n");
    wt.run(&["ontology", "generate"])
        .assert_code(1)
        .assert_stderr_contains("unknown section \"mapping\"")
        .assert_stderr_contains(
            "valid sections: classes, families, properties, relations, schemes, mappings, renames, external, source",
        );
    assert_eq!(wt.read(MODULE), full_module);
}

#[test]
fn generate_without_a_vendored_copy_explains_what_is_missing() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));

    wt.run(&["ontology", "generate"])
        .assert_code(1)
        .assert_stderr_contains("this project has no vendored ontology")
        .assert_stderr_contains("a checkout of the ontology's source repository")
        .assert_stderr_contains("A wiki that does not use an ontology needs neither command");

    let output = wt.run(&["ontology", "generate", "--check", "--format", "json"]);
    output
        .assert_code(1)
        .assert_json("/schema_version", "ontology_generate_v1")
        .assert_json("/status", "not_ready")
        .assert_json("/export_ready", false)
        .assert_json("/commit", Value::Null)
        .assert_json("/written", false);
    assert!(
        output
            .json_str("/message")
            .contains("`wikitool ontology export --from <dir>` produces it"),
        "{}",
        output.json_str("/message")
    );
    wt.assert_missing(EXPORT);
    wt.assert_missing(MODULE);
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create directory");
    for entry in fs::read_dir(source).expect("read directory") {
        let entry = entry.expect("directory entry");
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy file");
        }
    }
}

/// The Remilia adapter declares `[ontology]`, but only the ontology commands read the vendored
/// copy: everything else works in a project that never exported one.
#[test]
fn the_remilia_adapter_works_without_a_vendored_ontology() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site_adapters/remilia-wiki"),
        &wt.path().join("site-adapter"),
    );
    wt.run(&[
        "init",
        "--no-network",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);

    wt.run(&["adapter", "inspect", "--format", "json"])
        .assert_code(0)
        .assert_json("/adapter/adapter_id", "remilia-wiki")
        .assert_json("/adapter/ontology/product", "remilia-wiki")
        .assert_json("/adapter/ontology/imports", "v0.1.0/imports");
    wt.run(&["config", "show", "--format", "json"])
        .assert_code(0)
        .assert_json("/adapter_path", "site-adapter/site-adapter.toml");

    fs::create_dir_all(wt.path().join("wiki_content/Main")).expect("content dir");
    fs::write(
        wt.path().join("wiki_content/Main/Example.wiki"),
        "{{SHORTDESC:An example}}\n'''Example''' is a page.\n\n== References ==\n{{Reflist}}\n",
    )
    .expect("article");
    wt.run(&["catalog", "build", "--format", "json"])
        .assert_code(0);
    wt.run(&[
        "article",
        "lint",
        "wiki_content/Main/Example.wiki",
        "--format",
        "json",
    ])
    .assert_code(0);
    wt.run(&["article", "scout", "Example", "--format", "json"])
        .assert_code(0);

    wt.assert_missing(EXPORT);
    wt.assert_missing(MODULE);
}
