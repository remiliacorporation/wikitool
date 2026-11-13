use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::runtime::{ResolvedPaths, ValueSource};

const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
const PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";
const DEFINITION: &str = "http://www.w3.org/2004/02/skos/core#definition";
const IMPORTS: &str = "v9.9.9/imports";

/// An ontology directory (`ontology/`) inside a git checkout whose path
/// contains spaces, committed once with the base files.
struct Fixture {
    temp: TempDir,
    root: PathBuf,
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::Builder::new()
            .prefix("ontology fixture ")
            .tempdir()
            .expect("tempdir");
        let root = temp.path().join("checkout with spaces");
        let dir = root.join("ontology");
        fs::create_dir_all(&dir).expect("ontology dir");
        fs::write(temp.path().join("empty.gitconfig"), "").expect("gitconfig");
        let fixture = Self { temp, root, dir };
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.name", "Ontology Fixture"]);
        fixture.git(&["config", "user.email", "fixture@example.invalid"]);
        fixture.git(&["config", "core.autocrlf", "false"]);
        fixture.git(&["config", "commit.gpgsign", "false"]);
        fs::write(fixture.root.join("README.md"), "outside the ontology\n").expect("readme");
        for (path, text) in base_files() {
            fixture.write(path, &text);
        }
        fixture.commit();
        fixture
    }

    /// Run git in the checkout, isolated from the developer's global and
    /// system configuration.
    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
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

    fn write(&self, relative: &str, text: &str) {
        let path = self.dir.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, text).expect("write fixture file");
    }

    fn commit(&self) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", "fixture"]);
    }

    fn export_with(&self, imports: &str, excluded: &[&str]) -> Result<Value> {
        let excluded: Vec<String> = excluded.iter().map(|prefix| prefix.to_string()).collect();
        export(
            &self.dir,
            &ExportOptions {
                product: "test-wiki",
                imports: Path::new(imports),
                excluded_rename_prefixes: &excluded,
                published_at: Some("https://ontology.example/"),
                local: Some(("wiki", "https://wiki.example/ns/")),
            },
        )
    }

    fn export(&self) -> Result<Value> {
        self.export_with(IMPORTS, &["rn", "rns"])
    }

    fn export_error(&self) -> String {
        format!("{:#}", self.export().expect_err("export should fail"))
    }
}

fn triple(subject: &str, predicate: &str, object: &str) -> String {
    format!("<{subject}> <{predicate}> {object} .\n")
}

fn base_files() -> Vec<(&'static str, String)> {
    let q5 = "http://www.wikidata.org/entity/Q5";
    let q6 = "http://www.wikidata.org/entity/Q6";
    let community = "http://rdfs.org/sioc/ns#Community";
    // Lines are ordered so that taking the first acceptable literal would
    // pick the wrong one every time.
    let wikidata = [
        "# A pinned extract.\n".to_string(),
        triple(q5, LABEL, "\"Mensch\"@de"),
        triple(q5, LABEL, "\"human (untagged)\""),
        triple(q5, LABEL, "\"human (British)\"@en-GB"),
        triple(q5, LABEL, "\"human \\U0001F600\"@en"),
        triple(q5, COMMENT, "\"a comment, not a definition\"@en"),
        triple(q5, DEFINITION, "\"common name of Homo sapiens\"@EN"),
        "_:b0 <http://www.w3.org/2000/01/rdf-schema#label> \"blank\"@en .\n".to_string(),
        triple(q6, LABEL, "\"not wanted\"@en"),
        // Unwanted subjects are not checked past the subject.
        format!("<{q6}> is not a triple\n"),
    ]
    .concat();
    let sioc = [
        triple(community, LABEL, "\"Community (rdfs)\"@en"),
        triple(community, PREF_LABEL, "\"Community\"@en"),
        triple(
            community,
            COMMENT,
            "\"An online community.\"^^<http://www.w3.org/2001/XMLSchema#string>",
        ),
    ]
    .concat();
    vec![
        (
            "registry/agent.jsonld",
            json!({
                "@context": "https://ontology.example/context.jsonld",
                "@id": "remilia:Agent",
                "@type": "OntologyClass",
                "label": "Agent",
                "ontologyFamily": ["remilia:AgentFamily"],
            })
            .to_string(),
        ),
        (
            "registry/person.jsonld",
            json!({
                "@id": "remilia:Person",
                "@type": "OntologySubclass",
                "label": "Person",
                "subClassOf": "remilia:Agent",
            })
            .to_string(),
        ),
        (
            "registry/family.jsonld",
            json!({ "@id": "remilia:AgentFamily", "@type": "OntologyFamily", "label": "Agents" })
                .to_string(),
        ),
        (
            "registry/founder.jsonld",
            json!({ "@id": "remilia:founder", "@type": "OntologyProperty", "label": "founder" })
                .to_string(),
        ),
        (
            "registry/member-of.jsonld",
            json!({ "@id": "remilia:memberOf", "@type": "OntologyRelationType", "label": "member of" })
                .to_string(),
        ),
        (
            "registry/vocation.jsonld",
            json!({
                "@id": "remilia:vocation",
                "@type": "OntologyScheme",
                "label": "vocation",
                "members": [
                    { "@id": "remilia:vocation/team", "@type": "Concept", "label": "team" },
                    { "@id": "remilia:vocation/studio", "@type": "Concept", "label": "studio" },
                ],
            })
            .to_string(),
        ),
        ("registry/notes.txt", "not a registry entry\n".to_string()),
        (
            "mappings.sssom.tsv",
            "# curie_map:\n#   remilia: https://ontology.example/\n\
             subject_id\tpredicate_id\tobject_id\tcomment\n\
             remilia:Agent\tskos:exactMatch\tprov:Agent\t\n\
             remilia:Agent\tskos:closeMatch\twd:Q24229398\tsecond mapping\n\
             remilia:Person\tskos:exactMatch\tsssom:NoTermFound\tnothing outside\n"
                .to_string(),
        ),
        (
            "renames.sssom.tsv",
            "subject_id\tpredicate_id\tobject_id\n\
             rn:OldAgent\tdcterms:isReplacedBy\tremilia:Agent\n\
             rns:old-scheme\tdcterms:isReplacedBy\tremilia:vocation\n\
             rnx:Kept\tdcterms:isReplacedBy\tremilia:Person\n\
             remilia:Human\tdcterms:isReplacedBy\tremilia:Person\n"
                .to_string(),
        ),
        (
            "v9.9.9/imports/product-uses.tsv",
            "# curie_map:\n#   wd: http://www.wikidata.org/entity/\n#   sioc: http://rdfs.org/sioc/ns#\n\
             product\tiri\tpin\n\
             test-wiki\twd:Q5\twikidata.nt\n\
             test-wiki\twd:Q7\twikidata.nt\n\
             test-wiki\tsioc:Community\tsioc.nt\n\
             other-wiki\twd:Q6\twikidata.nt\n"
                .to_string(),
        ),
        ("v9.9.9/imports/wikidata.nt", wikidata),
        ("v9.9.9/imports/sioc.nt", sioc),
    ]
}

#[test]
fn export_builds_the_vendored_copy_from_one_commit() {
    let fixture = Fixture::new();
    let export = fixture.export().expect("export");

    // Every registry section is present, keyed by id, without JSON-LD keys.
    assert_eq!(
        export["classes"],
        json!({
            "remilia:Agent": { "label": "Agent", "ontologyFamily": ["remilia:AgentFamily"] },
            "remilia:Person": { "label": "Person", "subClassOf": "remilia:Agent" },
        })
    );
    assert_eq!(export["families"]["remilia:AgentFamily"]["label"], "Agents");
    assert_eq!(export["properties"]["remilia:founder"]["label"], "founder");
    assert_eq!(
        export["relations"]["remilia:memberOf"]["label"],
        "member of"
    );
    assert_eq!(
        export["schemes"]["remilia:vocation"]["members"],
        json!({
            "remilia:vocation/studio": { "label": "studio" },
            "remilia:vocation/team": { "label": "team" },
        })
    );

    // Mappings keep file order per subject and drop sssom:NoTermFound.
    assert_eq!(
        export["mappings"],
        json!({
            "remilia:Agent": [
                { "predicate": "skos:exactMatch", "object": "prov:Agent" },
                { "predicate": "skos:closeMatch", "object": "wd:Q24229398" },
            ],
        })
    );

    // Renames drop exactly the excluded CURIE prefixes.
    assert_eq!(
        export["renames"],
        json!({ "remilia:Human": "remilia:Person", "rnx:Kept": "remilia:Person" })
    );

    // External terms: only this product's, labelled by the selection rule.
    assert_eq!(
        export["external"],
        json!({
            "sioc:Community": {
                "iri": "http://rdfs.org/sioc/ns#Community",
                "label": "Community",
                "definition": "An online community.",
            },
            "wd:Q5": {
                "iri": "http://www.wikidata.org/entity/Q5",
                "label": "human \u{1F600}",
                "definition": "common name of Homo sapiens",
            },
            "wd:Q7": { "iri": "http://www.wikidata.org/entity/Q7" },
        })
    );

    assert_eq!(
        export["source"],
        json!({
            "commit": fixture.git(&["rev-parse", "HEAD"]),
            "product": "test-wiki",
            "ontology": "https://ontology.example/",
            "local": { "wiki": "https://wiki.example/ns/" },
        })
    );

    // Serialized keys come out sorted, so the vendored file diffs stably.
    let text = serde_json::to_string_pretty(&export).expect("serialize");
    let top: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("  \"")?.split_once('"'))
        .map(|(key, _)| key)
        .collect();
    assert_eq!(
        top,
        [
            "classes",
            "external",
            "families",
            "mappings",
            "properties",
            "relations",
            "renames",
            "schemes",
            "source"
        ]
    );
}

#[test]
fn rename_exclusion_is_empty_by_default() {
    let fixture = Fixture::new();
    let export = fixture.export_with(IMPORTS, &[]).expect("export");
    assert_eq!(export["renames"]["rn:OldAgent"], "remilia:Agent");
    assert_eq!(export["renames"]["rns:old-scheme"], "remilia:vocation");
}

#[test]
fn export_refuses_uncommitted_and_ignored_files() {
    let fixture = Fixture::new();
    // A change outside the ontology directory does not matter.
    fs::write(fixture.root.join("README.md"), "edited\n").expect("edit readme");
    fixture.export().expect("export with a dirty file outside");

    fixture.write("registry/agent.jsonld", "{}");
    assert!(fixture.export_error().contains("uncommitted changes"));
    fixture.git(&["checkout", "--", "ontology"]);

    // `git status` does not show ignored files, but the commit lacks them.
    fixture.write(".gitignore", "registry/ignored.jsonld\n");
    fixture.commit();
    fixture.write(
        "registry/ignored.jsonld",
        &json!({ "@id": "remilia:Ignored", "@type": "OntologyClass" }).to_string(),
    );
    let error = fixture.export_error();
    assert!(
        error.contains("registry/ignored.jsonld is not committed"),
        "{error}"
    );
}

#[test]
fn export_rejects_inconsistent_registries() {
    let fixture = Fixture::new();
    fixture.write(
        "registry/zz-agent-again.jsonld",
        &json!({ "@id": "remilia:Agent", "@type": "OntologySubclass" }).to_string(),
    );
    fixture.commit();
    let error = fixture.export_error();
    assert!(
        error.contains("zz-agent-again.jsonld: remilia:Agent is defined twice"),
        "{error}"
    );

    let fixture = Fixture::new();
    fixture.write(
        "registry/odd.jsonld",
        &json!({ "@id": "remilia:Odd", "@type": "OntologyThing" }).to_string(),
    );
    fixture.commit();
    let error = fixture.export_error();
    assert!(
        error.contains("odd.jsonld: unknown @type OntologyThing"),
        "{error}"
    );
}

#[test]
fn imports_directory_is_explicit_and_stays_inside_the_ontology() {
    let fixture = Fixture::new();
    let error = format!(
        "{:#}",
        fixture
            .export_with("v0.1.0/imports", &[])
            .expect_err("missing")
    );
    assert!(
        error.contains("v0.1.0/imports/product-uses.tsv is not committed"),
        "{error}"
    );
    for escaping in ["../imports", "/abs/imports"] {
        let error = format!(
            "{:#}",
            fixture.export_with(escaping, &[]).expect_err("escaping")
        );
        assert!(error.contains("must be relative"), "{escaping}: {error}");
    }
}

#[test]
fn malformed_extract_lines_about_wanted_terms_fail_with_their_location() {
    let fixture = Fixture::new();
    let mut extract =
        fs::read_to_string(fixture.dir.join(IMPORTS).join("wikidata.nt")).expect("read extract");
    // Line 11: a JSON escape, which N-Triples does not have.
    extract.push_str(&triple(
        "http://www.wikidata.org/entity/Q7",
        LABEL,
        "\"bad \\x41\"@en",
    ));
    fixture.write(&format!("{IMPORTS}/wikidata.nt"), &extract);
    fixture.commit();
    let error = fixture.export_error();
    assert!(error.contains("v9.9.9/imports/wikidata.nt:11:"), "{error}");

    // A line whose subject cannot be read may be about any term.
    let fixture = Fixture::new();
    fixture.write(&format!("{IMPORTS}/sioc.nt"), "wd:Q5 rdfs:label \"x\" .\n");
    fixture.commit();
    let error = fixture.export_error();
    assert!(error.contains("v9.9.9/imports/sioc.nt:1:"), "{error}");
}

#[test]
fn a_crlf_checkout_exports_the_same_copy() {
    let fixture = Fixture::new();
    let lf = fixture.export().expect("LF export");
    // Check out the same commit again with CRLF working files, as a Windows
    // clone with core.autocrlf does.
    fixture.git(&["config", "core.autocrlf", "true"]);
    fs::remove_dir_all(&fixture.dir).expect("remove working files");
    fixture.git(&["checkout", "--", "ontology"]);
    let mappings = fs::read(fixture.dir.join("mappings.sssom.tsv")).expect("read");
    assert!(mappings.windows(2).any(|pair| pair == b"\r\n"), "not CRLF");
    assert_eq!(fixture.export().expect("CRLF export"), lf);
}

#[test]
fn tsv_rows_need_one_cell_per_column() {
    let path = Path::new("table.tsv");
    let table = Tsv::parse(path, "# meta\r\na\tb\tc\r\n\r\n1\t\t\r\n").expect("parse");
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0].line, 4);
    assert_eq!(table.rows[0].cells, ["1", "", ""]);

    let short = Tsv::parse(path, "a\tb\tc\n1\t2\t3\n1\t2\n").expect_err("short");
    assert!(
        format!("{short:#}").contains("table.tsv:3: the row has 2 cells but the header has 3"),
        "{short:#}"
    );
    let long = Tsv::parse(path, "a\tb\n1\t2\t3\n").expect_err("long");
    assert!(format!("{long:#}").contains("table.tsv:2:"), "{long:#}");

    let table = Tsv::parse(path, "a\tb\n").expect("header only");
    let missing = table.columns(["a", "z"]).expect_err("missing column");
    assert!(format!("{missing:#}").contains("table.tsv has no z column"));
    assert!(Tsv::parse(path, "# only metadata\n").is_err());
    assert!(Tsv::parse(path, "a\ta\n").is_err());
}

#[test]
fn curie_map_reads_the_comment_header() {
    let map =
        read_curie_map("# curie_map:\n#  wd: http://www.wikidata.org/entity/\niri\tproduct\n");
    assert_eq!(
        expand("wd:Q5", &map).as_deref(),
        Some("http://www.wikidata.org/entity/Q5")
    );
}

#[test]
fn lua_output_is_sorted_and_escaped() {
    let value = json!({
        "source": { "commit": "abc" },
        "b": ["x", "y\"z"],
        "a": { "label": "line\nbreak", "skip": null },
    });
    let lua = generate_module(&value).expect("module");
    assert!(lua.starts_with(
        "-- Generated by `wikitool ontology generate` from vendor/ontology/ontology.json\n\
         -- at ontology commit abc. Do not edit by hand.\nreturn {\n"
    ));
    let body = lua.split_once("return ").expect("return").1;
    assert!(body.find("[\"a\"]").unwrap() < body.find("[\"b\"]").unwrap());
    assert!(body.contains("[\"label\"] = \"line\\nbreak\""));
    assert!(body.contains("{\"x\", \"y\\\"z\"}"));
    assert!(!body.contains("skip"));
    full_moon::parse(&lua).expect("valid Lua");
}

#[test]
fn lua_strings_use_lua_5_1_escapes() {
    let mut out = String::new();
    write_lua_string(
        &mut out,
        "\u{0}\u{1}2\u{7f}\u{1b}[\t\r\n\\\"é \u{1F600} \u{2028}",
    );
    // \ddd takes at most three digits, so "\0012" is U+0001 then "2".
    assert_eq!(
        out,
        "\"\\000\\0012\\127\\027[\\t\\r\\n\\\\\\\"é \u{1F600} \u{2028}\""
    );
    full_moon::parse(&format!("return {out}")).expect("valid Lua");
}

#[test]
fn lua_rejects_null_in_an_array() {
    let value = json!({
        "source": { "commit": "abc" },
        "classes": { "remilia:a/b": { "list": ["x", null] } },
    });
    let error = generate_module(&value).expect_err("null in array");
    assert!(
        error
            .to_string()
            .contains("null at JSON pointer /classes/remilia:a~1b/list/1"),
        "{error}"
    );
}

#[test]
fn module_path_is_where_sync_reads_the_module_title() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let paths = ResolvedPaths {
        project_root: root.to_path_buf(),
        wiki_content_dir: root.join("wiki_content"),
        templates_dir: root.join("templates"),
        state_dir: root.join(".wikitool"),
        data_dir: root.join(".wikitool/data"),
        db_path: root.join(".wikitool/data/wikitool.db"),
        config_path: root.join(".wikitool/config.toml"),
        parser_config_path: root.join(".wikitool/parser-config.json"),
        root_source: ValueSource::Default,
        data_source: ValueSource::Default,
        config_source: ValueSource::Default,
    };
    let relative = module_relative_path(&paths).expect("module path");
    assert_eq!(relative, "templates/ontology/Module_Ontology/data.lua");
    assert_eq!(
        crate::filesystem::relative_path_to_title(&paths, &relative).expect("title"),
        MODULE_TITLE
    );
}
