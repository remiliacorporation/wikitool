//! Ontology-aware lint, scout, check and report against a small synthetic ontology: a vendored
//! export with a handful of classes, properties, a relation, a value list and a rename, and
//! the wiki's own `Module:Ontology/local` and `Module:Ontology/display` in the templates mirror.
//! A wiki whose adapter has no `[ontology]` gets none of it.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use wikitest::harness::Project;

const EXPORT: &str = "vendor/ontology/ontology.json";
const LOCAL: &str = "templates/ontology/Module_Ontology/local.lua";
const DISPLAY: &str = "templates/ontology/Module_Ontology/display.lua";

fn export() -> Value {
    json!({
        "classes": {
            "prov:Agent": { "label": "Agent" },
            "remilia:Idea": { "label": "Idea" },
            "remilia:Artwork": { "label": "Artwork" },
            "schema:Event": { "label": "Event" }
        },
        "families": {},
        "properties": {
            "remilia:artMedium": {
                "label": "Art medium", "valueType": "controlled-term", "cardinality": "one",
                "valueScheme": "remilia:art-medium/", "appliesTo": ["remilia:Artwork"]
            },
            "remilia:releaseDate": {
                "label": "Release date", "valueType": "date", "cardinality": "one",
                "appliesTo": ["remilia:Artwork"]
            },
            "remilia:baseClock": {
                "label": "Base clock", "valueType": "quantity", "cardinality": "one",
                "allowedUnits": ["Hz", "GHz"], "appliesTo": ["remilia:Artwork"]
            },
            "wdt:P276": {
                "label": "location", "valueType": "canonical-object", "cardinality": "one",
                "appliesTo": ["schema:Event"]
            },
            "wdt:P212": {
                "label": "ISBN-13", "valueType": "external-id", "cardinality": "one",
                "appliesTo": ["remilia:Artwork"]
            },
            "dcterms:source": {
                "label": "source", "valueType": "canonical-object", "cardinality": "one",
                "isQualifier": true
            }
        },
        "relations": {
            "remilia:memberOf": {
                "label": "Member of", "inverseLabel": "Has member",
                "domainClasses": ["prov:Agent"], "rangeClasses": ["prov:Agent"]
            }
        },
        "schemes": {
            "remilia:art-medium/": {
                "label": "Art medium",
                "members": {
                    "remilia:art-medium/painting": { "label": "painting" },
                    "remilia:art-medium/digital": { "label": "digital art" }
                }
            }
        },
        "mappings": {},
        "renames": { "remilia:OldArtwork": "remilia:Artwork" },
        "external": { "wd:Q5": { "iri": "http://www.wikidata.org/entity/Q5", "label": "human" } },
        "source": { "commit": "fixture-commit", "product": "test-wiki" }
    })
}

const LOCAL_MODULE: &str = r#"-- The wiki's own terms.
return {
	classes = {
		['wd:Q5'] = { label = 'Human', subClassOf = 'prov:Agent' },
		['wiki:Thesis'] = { label = 'Thesis', subClassOf = 'remilia:Idea' },
		['remilia:Idea'] = { label = 'Idea (local copy)' },
	},
	properties = {
		['wdt:P569'] = {
			label = 'Date of birth',
			valueType = 'date',
			cardinality = 'one',
			appliesTo = { 'wd:Q5' },
		},
		['wiki:location'] = {
			label = 'Location',
			valueType = 'text',
			cardinality = 'one',
			appliesTo = { 'remilia:Artwork' },
		},
	},
	mappings = {
		['wd:Q5'] = { { predicate = 'skos:exactMatch', object = 'wd:Q5' } },
		['wdt:P569'] = { { predicate = 'skos:exactMatch', object = 'wdt:P569' } },
		['wiki:location'] = { { predicate = 'skos:closeMatch', object = 'sssom:NoTermFound' } },
	},
}
"#;

const DISPLAY_MODULE: &str = r#"return {
	fields = {
		birth_date = { property = 'wdt:P569', label = 'Born' },
		affiliation = { property = 'remilia:memberOf', label = 'Affiliation' },
		released = { property = 'remilia:releaseDate', label = 'Released' },
		medium = { property = 'remilia:artMedium', label = 'Medium' },
		clock = { property = 'remilia:baseClock', label = 'Clock' },
		isbn = { property = 'wdt:P212', label = 'ISBN' },
		plain = { label = 'Plain' },
		-- Its own property applies only to events; the artwork kind
		-- overrides it with one that applies.
		location = { property = 'wdt:P276', label = 'Location' },
		movement = { property = 'wdt:P135', label = 'Movement', category = '%s' },
		member_of = { property = 'remilia:memberOf', label = 'Member of', category = '%s' },
	},
	types = {
		person = {
			class = 'wd:Q5',
			sections = {
				{ fields = { 'birth_date', 'affiliation', 'released', { field = 'plain', property = 'wiki:missing' } } },
			},
		},
		artwork = {
			class = 'remilia:Artwork',
			sections = {
				{ fields = { 'medium', 'released', 'clock', 'isbn', 'plain', { field = 'location', property = 'wiki:location' } } },
			},
		},
	},
}
"#;

const TYPED_ARTICLE: &str = "\
{{SHORTDESC:A typed example}}
{{Instance of|wd:Q5|remilia:OldArtwork|wd:Q999|movement=New Net Art|colour=red|wikidata=42}}
{{Infobox person|birth_date=May 12, 2019|released=2020|affiliation=[[Remilia]]}}
{{Infobox|type=artwork|medium=[[Painting|painting]]|released=sometime<ref>{{Cite web|title=x}}</ref>|clock=3.2 GHz|isbn=978 0|plain=x|location=Paris}}
{{Infobox|type=spaceship}}
'''Typed''' is an example.

== References ==
{{Reflist}}
";

const LOCAL_TERMS_ARTICLE: &str = "\
{{SHORTDESC:A thesis}}
{{Instance of|wiki:Thesis|remilia:Idea}}
'''Thesis''' is an example.

== References ==
{{Reflist}}
";

/// Ids are compared as the module sees them: comments gone, and a later argument for a
/// position wins, so `1=wd:Q5` replaces the unknown id before it.
const COMMENTED_ARTICLE: &str = "\
{{SHORTDESC:Commented}}
{{Instance of|wd:Q5 <!-- human -->|wikidata=Q42<!-- the item -->}}
{{Instance of|wd:Q999|1=wd:Q5}}
'''Commented''' is an example.

== References ==
{{Reflist}}
";

fn generic_adapter() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site_adapters/generic/site-adapter.toml"),
    )
    .expect("generic adapter")
}

/// A project whose adapter has `[ontology]`, with `lint` added to `[lint]`.
fn project(lint: &str, with_export: bool, with_modules: bool) -> Project<'static> {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let adapter = generic_adapter().replacen("[lint]\n", &format!("[lint]\n{lint}"), 1)
        + "\n[ontology]\n\
           product = \"test-wiki\"\n\
           source = \"https://ontology.example/\"\n\
           local_prefix = \"wiki\"\n\
           local_namespace = \"https://wiki.example/ns/\"\n\
           class_template = \"Template:Instance of\"\n\
           typed_infobox_template = \"Template:Infobox\"\n\
           date_formats = [\"%B %-d, %Y\"]\n\
           [ontology.infobox_types]\n\
           \"Template:Infobox person\" = \"person\"\n";
    write(&wt, "site-adapter/site-adapter.toml", &adapter);
    init(&wt);
    if with_export {
        write(
            &wt,
            EXPORT,
            &serde_json::to_string_pretty(&export()).expect("json"),
        );
    }
    if with_modules {
        write(&wt, LOCAL, LOCAL_MODULE);
        write(&wt, DISPLAY, DISPLAY_MODULE);
    }
    write(&wt, "wiki_content/Main/Typed.wiki", TYPED_ARTICLE);
    write(&wt, "wiki_content/Main/Thesis.wiki", LOCAL_TERMS_ARTICLE);
    write(&wt, "wiki_content/Main/Commented.wiki", COMMENTED_ARTICLE);
    wt
}

fn write(wt: &Project<'_>, relative: &str, text: &str) {
    let path = wt.path().join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, text).expect("write fixture");
}

fn init(wt: &Project<'_>) {
    wt.run(&[
        "init",
        "--no-network",
        "--api-url",
        "https://wikitest.invalid/api.php",
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);
}

fn lint(wt: &Project<'_>, relative: &str) -> Value {
    wt.run(&["article", "lint", relative, "--format", "json"])
        .json()
}

fn ontology_issues(report: &Value) -> Vec<(String, String, String)> {
    report["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .filter(|issue| {
            issue["rule_id"]
                .as_str()
                .unwrap_or("")
                .starts_with("ontology.")
        })
        .map(|issue| {
            (
                issue["rule_id"].as_str().unwrap_or("").to_string(),
                issue["severity"].as_str().unwrap_or("").to_string(),
                issue["message"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

fn assert_issue(issues: &[(String, String, String)], code: &str, needle: &str) {
    assert!(
        issues
            .iter()
            .any(|(rule, _, message)| rule == code && message.contains(needle)),
        "no {code} naming {needle:?} in {issues:#?}"
    );
}

#[test]
fn lint_checks_classes_fields_and_values_as_warnings_by_default() {
    let wt = project("", true, true);
    let report = lint(&wt, "wiki_content/Main/Typed.wiki");
    let issues = ontology_issues(&report);
    // A property that does not apply is the display module's defect, which `ontology check`
    // reports; on an article it is only a suggestion. Everything else is a warning.
    for (rule, severity, message) in &issues {
        let expected = if rule == "ontology.property_not_applicable" {
            "suggestion"
        } else {
            "warning"
        };
        assert_eq!(severity, expected, "{rule}: {message}");
    }
    assert_issue(
        &issues,
        "ontology.renamed_term",
        "remilia:OldArtwork, which the ontology renamed to remilia:Artwork",
    );
    assert_issue(
        &issues,
        "ontology.unknown_class",
        "names wd:Q999, which is not a class",
    );
    assert_issue(&issues, "ontology.unknown_field", "the field colour");
    assert_issue(&issues, "ontology.malformed_wikidata_id", "wikidata=42");
    assert_issue(
        &issues,
        "ontology.property_not_applicable",
        "remilia:releaseDate, which does not apply to wd:Q5 (Human)",
    );
    assert_issue(
        &issues,
        "ontology.value_type_mismatch",
        "\"sometime\" is not an ISO 8601 date",
    );
    assert_issue(
        &issues,
        "ontology.value_type_mismatch",
        "\"978 0\" is not an identifier",
    );
    assert_issue(&issues, "ontology.unknown_type", "\"spaceship\"");
    // A relation field and a field with no property are not reported; a human date in a listed
    // format, a scheme member by label, a quantity in an allowed unit, and a relation whose domain
    // is an ancestor of the class all pass.
    assert_eq!(issues.len(), 8, "{issues:#?}");
    assert_eq!(report["ontology"]["ontology_commit"], "fixture-commit");
    assert_eq!(
        report["ontology"]["checks_run"],
        json!([
            "class_statements",
            "infobox_fields",
            "field_values",
            "term_definitions"
        ])
    );
    assert_eq!(report["ontology"]["skipped"], json!([]));

    // A field the kind gives its own property (location: wiki:location for artworks) is
    // checked against that property, not the field's (wdt:P276, events only).
    assert!(
        issues
            .iter()
            .all(|(_, _, message)| !message.contains("location")),
        "{issues:#?}"
    );

    let commented = lint(&wt, "wiki_content/Main/Commented.wiki");
    assert_eq!(ontology_issues(&commented), Vec::new());
    let thesis = lint(&wt, "wiki_content/Main/Thesis.wiki");
    let issues = ontology_issues(&thesis);
    assert_issue(
        &issues,
        "ontology.unmapped_local_term",
        "wiki:Thesis in Module:Ontology/local has no mappings entry",
    );
    assert_issue(
        &issues,
        "ontology.local_shadows_shared",
        "defines remilia:Idea among its classes",
    );
    assert_eq!(issues.len(), 2, "{issues:#?}");
}

#[test]
fn the_adapter_sets_the_level_and_can_turn_the_checks_off() {
    let wt = project("ontology_typing = \"error\"\n", true, true);
    let issues = ontology_issues(&lint(&wt, "wiki_content/Main/Typed.wiki"));
    assert!(!issues.is_empty());
    assert!(
        issues
            .iter()
            .filter(|(rule, _, _)| rule != "ontology.property_not_applicable")
            .all(|(_, severity, _)| severity == "error")
    );

    let off = project("ontology_typing = \"ignore\"\n", true, true);
    let report = lint(&off, "wiki_content/Main/Typed.wiki");
    assert!(ontology_issues(&report).is_empty());
    assert!(report.get("ontology").is_none());
}

#[test]
fn missing_export_or_modules_skip_the_checks_with_the_reason() {
    let no_export = project("", false, true);
    let report = lint(&no_export, "wiki_content/Main/Typed.wiki");
    assert!(ontology_issues(&report).is_empty());
    assert_eq!(report["ontology"]["checks_run"], json!([]));
    assert!(
        report["ontology"]["skipped"][0]["reason"]
            .as_str()
            .expect("reason")
            .contains("no vendored ontology at vendor/ontology/ontology.json")
    );

    // Scout names the reason too, rather than leaving the object out.
    no_export
        .run(&["catalog", "build", "--format", "json"])
        .assert_code(0);
    let scout = no_export.run(&["article", "scout", "Typed", "--format", "json"]);
    scout
        .assert_code(0)
        .assert_json("/ontology/probable_class_count", 0);
    assert!(
        scout
            .json_str("/ontology/skipped/0/reason")
            .contains("no vendored ontology")
    );

    let no_modules = project("", true, false);
    let report = lint(&no_modules, "wiki_content/Main/Typed.wiki");
    assert!(ontology_issues(&report).is_empty());
    let reason = report["ontology"]["skipped"][0]["reason"]
        .as_str()
        .expect("reason");
    assert!(
        reason.contains(
            "Module:Ontology/local is not in the templates mirror at templates/ontology/Module_Ontology/local.lua"
        ),
        "{reason}"
    );
    no_modules
        .run(&["ontology", "check", "--format", "json"])
        .assert_code(1)
        .assert_json("/status", "incomplete");
}

#[test]
fn a_wiki_without_an_ontology_section_gets_no_checks() {
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    write(&wt, "site-adapter/site-adapter.toml", &generic_adapter());
    init(&wt);
    write(
        &wt,
        EXPORT,
        &serde_json::to_string_pretty(&export()).expect("json"),
    );
    write(&wt, LOCAL, LOCAL_MODULE);
    write(&wt, DISPLAY, DISPLAY_MODULE);
    write(&wt, "wiki_content/Main/Typed.wiki", TYPED_ARTICLE);
    let report = lint(&wt, "wiki_content/Main/Typed.wiki");
    assert!(ontology_issues(&report).is_empty());
    assert!(report.get("ontology").is_none());
    wt.run(&["ontology", "check", "--format", "json"])
        .assert_code(1)
        .assert_json("/status", "not_ready");
    wt.run(&["catalog", "build", "--format", "json"])
        .assert_code(0);
    let scout = wt.run(&[
        "article", "scout", "Typed", "--format", "json", "--view", "full",
    ]);
    scout.assert_code(0);
    assert!(scout.json().get("ontology").is_none());
}

#[test]
fn check_reports_module_findings_and_fails() {
    let wt = project("", true, true);
    let output = wt.run(&["ontology", "check", "--format", "json"]);
    output
        .assert_code(1)
        .assert_json("/schema_version", "ontology_check_v1")
        .assert_json("/status", "findings")
        .assert_json("/commit", "fixture-commit")
        .assert_json("/finding_count", 5)
        .assert_stderr_contains("ontology check found 5 finding(s)");
    let findings: Vec<(String, String)> = output
        .json_array("/findings")
        .iter()
        .map(|finding| {
            (
                finding["code"].as_str().unwrap_or("").to_string(),
                finding["term"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    for expected in [
        ("ontology.unmapped_local_term", "wiki:Thesis"),
        ("ontology.local_shadows_shared", "remilia:Idea"),
        ("ontology.unresolved_field_property", "wdt:P135"),
        ("ontology.property_not_applicable", "remilia:releaseDate"),
        ("ontology.unresolved_field_property", "wiki:missing"),
    ] {
        assert!(
            findings.contains(&(expected.0.to_string(), expected.1.to_string())),
            "{expected:?} in {findings:?}"
        );
    }

    // A module that is code rather than data is refused with its line.
    write(&wt, LOCAL, "local x = 1\nreturn { classes = {} }\n");
    let output = wt.run(&["ontology", "check", "--format", "json"]);
    output.assert_code(1).assert_json("/status", "incomplete");
    let error = output.json_str("/modules/0/error");
    assert!(
        error.contains("Module:Ontology/local:1: a data module holds only"),
        "{error}"
    );
}

#[test]
fn report_counts_term_usage_unmapped_fields_failed_values_and_unknown_classes() {
    let wt = project("", true, true);
    let output = wt.run(&["ontology", "report", "--format", "json"]);
    output
        .assert_code(0)
        .assert_json("/schema_version", "ontology_report_v1")
        .assert_json("/page_count", 3)
        .assert_json("/typed_page_count", 3);
    let report = output.json();
    let term = |id: &str| {
        report["local_terms"]
            .as_array()
            .expect("local terms")
            .iter()
            .find(|term| term["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("{id} missing"))
    };
    assert_eq!(term("wd:Q5")["page_count"], 2);
    assert_eq!(
        term("wd:Q5")["example_titles"],
        json!(["Commented", "Typed"])
    );
    assert_eq!(term("wiki:location")["page_count"], 1);
    assert_eq!(term("wdt:P569")["page_count"], 1);
    assert_eq!(term("wiki:Thesis")["page_count"], 1);
    assert_eq!(term("wiki:Thesis")["mapped"], false);
    assert_eq!(term("remilia:Idea")["shadowed"], true);
    assert_eq!(
        report["fields_without_property"],
        json!([{ "field": "plain", "label": "Plain", "page_count": 1, "example_titles": ["Typed"] }])
    );
    let failures = report["value_failures"].as_array().expect("failures");
    assert_eq!(failures.len(), 2, "{failures:#?}");
    let release = failures
        .iter()
        .find(|failure| failure["property"] == "remilia:releaseDate")
        .expect("release date failure");
    assert_eq!(release["value_type"], "date");
    assert_eq!(release["failure_count"], 1);
    assert_eq!(release["examples"][0]["title"], "Typed");
    assert_eq!(
        report["unknown_classes"],
        json!([{ "id": "wd:Q999", "page_count": 1, "example_titles": ["Typed"] }])
    );
}

#[test]
fn scout_lists_probable_classes_and_what_applies_to_them() {
    let wt = project("", true, true);
    write(
        &wt,
        "wiki_content/Main/Draft_artwork.wiki",
        "{{Infobox|type=artwork|medium=painting}}\n'''Draft artwork''' links [[Typed]].\n",
    );
    wt.run(&["catalog", "build", "--format", "json"])
        .assert_code(0);

    let full = wt.run(&[
        "article",
        "scout",
        "Draft artwork",
        "--stub-path",
        "wiki_content/Main/Draft_artwork.wiki",
        "--format",
        "json",
        "--view",
        "full",
    ]);
    full.assert_code(0)
        .assert_json("/ontology/schema_version", "ontology_scout_v1")
        .assert_json("/ontology/evidence_source", "stub")
        .assert_json("/ontology/probable_classes/0/id", "remilia:Artwork")
        .assert_json("/ontology/probable_classes/0/label", "Artwork")
        .assert_json("/ontology/probable_classes/0/evidence/0/via", "infobox")
        .assert_json(
            "/ontology/probable_classes/0/evidence/0/infobox_type",
            "artwork",
        );
    let properties = full.json_array("/ontology/probable_classes/0/properties");
    let medium = properties
        .iter()
        .find(|property| property["id"] == "remilia:artMedium")
        .expect("art medium applies");
    assert_eq!(medium["field"], "medium");
    assert_eq!(medium["value_type"], "controlled-term");
    assert_eq!(medium["value_scheme"], "remilia:art-medium/");
    // The artwork kind carries wiki:location in its location field, overriding the field's own.
    let location = properties
        .iter()
        .find(|property| property["id"] == "wiki:location")
        .expect("wiki:location applies");
    assert_eq!(location["field"], "location");
    assert_eq!(location["local"], true);
    // A qualifier with no appliesTo applies to no class.
    assert!(
        properties
            .iter()
            .all(|property| property["id"] != "dcterms:source")
    );
    assert!(
        properties
            .iter()
            .all(|property| property["id"] != "wdt:P569")
    );

    let brief = wt.run(&[
        "article",
        "scout",
        "Draft artwork",
        "--stub-path",
        "wiki_content/Main/Draft_artwork.wiki",
        "--format",
        "json",
    ]);
    brief
        .assert_code(0)
        .assert_json("/ontology/probable_class_count", 1)
        .assert_json("/ontology/top_classes/0/id", "remilia:Artwork")
        .assert_json("/ontology/top_classes/0/property_count", 5)
        .assert_json("/ontology/top_classes/0/relation_count", 0);

    // Without a stub, an existing page's statements are the evidence: Typed is a human.
    let existing = wt.run(&[
        "article", "scout", "Typed", "--format", "json", "--view", "full",
    ]);
    existing
        .assert_code(0)
        .assert_json("/ontology/evidence_source", "existing_page")
        .assert_json("/ontology/probable_classes/0/id", "wd:Q5")
        .assert_json("/ontology/probable_classes/0/ancestors/0/id", "prov:Agent");
    let relations = existing.json_array("/ontology/probable_classes/0/relations");
    assert_eq!(relations[0]["id"], "remilia:memberOf");
    assert_eq!(relations[0]["applies_through"], "prov:Agent");
    // The person kind lists affiliation for remilia:memberOf, so it wins over member_of.
    assert_eq!(relations[0]["field"], "affiliation");
}
