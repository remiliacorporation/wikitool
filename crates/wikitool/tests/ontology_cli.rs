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
