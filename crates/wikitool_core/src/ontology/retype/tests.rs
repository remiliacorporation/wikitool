use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::proof::{run_and_check, synthetic_ontology, synthetic_ontology_edited};
use super::*;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ontology/retype/fixtures")
}

fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(format!("{name}.wiki")))
        .unwrap_or_else(|error| panic!("fixture {name}: {error}"))
}

fn expected(name: &str) -> Option<String> {
    fs::read_to_string(fixtures().join(format!("{name}.expected.wiki"))).ok()
}

fn item(plan: Value) -> RetypePlanItem {
    let mut plan = plan;
    plan["title"] = json!("Fixture");
    serde_json::from_value(plan).expect("plan item")
}

struct Case {
    /// The fixture's file name without `.wiki`.
    input: &'static str,
    plan: Value,
    /// `changed`, `unchanged` or `refused`.
    status: &'static str,
    refusals: &'static [&'static str],
}

fn cases() -> Vec<Case> {
    let case = |input, plan, status, refusals| Case {
        input,
        plan,
        status,
        refusals,
    };
    vec![
        case(
            "insert-after-short-description",
            json!({"classes": ["test:Kind"], "remove_categories": ["Memes"], "add_categories": ["Added topics"]}),
            "changed",
            &[],
        ),
        case(
            "insert-before-infobox",
            json!({"classes": ["test:Kind", "test:Other"]}),
            "changed",
            &[],
        ),
        case(
            "typed-infobox-states-the-class",
            json!({"classes": ["test:Person"]}),
            "unchanged",
            &[],
        ),
        case(
            "insert-at-top",
            json!({"classes": ["test:Kind"], "relations": {"movement": ["New Net Art"], "member_of": []}, "wikidata": "Q42"}),
            "changed",
            &[],
        ),
        case(
            "replace-multiline-call",
            json!({"classes": ["test:Kind", "test:Other"], "relations": {"movement": ["New Net Art"], "member_of": []}, "wikidata": "Q5"}),
            "changed",
            &[],
        ),
        case(
            "equivalent-call-unchanged",
            json!({"classes": ["test:Kind"], "relations": {"movement": ["New Net Art"]}}),
            "unchanged",
            &[],
        ),
        case(
            "redirect",
            json!({"remove_categories": ["Memes"]}),
            "refused",
            &[REFUSED_REDIRECT],
        ),
        case(
            "parse-failure",
            json!({"classes": ["test:Kind"]}),
            "refused",
            &[REFUSED_PARSE_FAILED],
        ),
        case(
            "plain",
            json!({"classes": ["test:Nope"]}),
            "refused",
            &[UNKNOWN_CLASS],
        ),
        case(
            "plain",
            json!({"classes": ["test:Old"]}),
            "refused",
            &[RENAMED_TERM],
        ),
        case(
            "plain",
            json!({"remove_categories": ["Absent"]}),
            "refused",
            &[REFUSED_REMOVAL_MISSING],
        ),
        case(
            "plain",
            json!({"remove_categories": ["Memes"], "add_categories": ["memes"]}),
            "refused",
            &[REFUSED_INVALID_PLAN],
        ),
        case(
            "plain",
            json!({"relations": {"movement": ["New Net Art"]}}),
            "refused",
            &[REFUSED_INVALID_PLAN],
        ),
        case(
            "plain",
            json!({"classes": ["test:Kind"], "add_categories": ["Kinds"], "drop_generated": true}),
            "refused",
            &[REFUSED_INVALID_PLAN],
        ),
        case(
            "plain",
            json!({"classes": ["test:Kind|x"]}),
            "refused",
            &[REFUSED_INVALID_PLAN],
        ),
        case(
            "plain",
            json!({"add_categories": ["Memes"]}),
            "unchanged",
            &[],
        ),
        case(
            "category-in-comment",
            json!({"remove_categories": ["Memes"]}),
            "refused",
            &[REFUSED_CATEGORY_NOT_EDITABLE],
        ),
        case(
            "category-in-nowiki",
            json!({"remove_categories": ["Memes"]}),
            "refused",
            &[REFUSED_CATEGORY_NOT_EDITABLE],
        ),
        case(
            "category-in-includeonly",
            json!({"remove_categories": ["Memes"]}),
            "refused",
            &[REFUSED_CATEGORY_NOT_EDITABLE],
        ),
        case(
            "category-in-noinclude",
            json!({"remove_categories": ["Memes"]}),
            "refused",
            &[REFUSED_CATEGORY_NOT_EDITABLE],
        ),
        case(
            "category-in-template",
            json!({"remove_categories": ["Memes"]}),
            "refused",
            &[REFUSED_CATEGORY_NOT_EDITABLE],
        ),
        case(
            "category-in-template",
            json!({"add_categories": ["Memes"]}),
            "refused",
            &[REFUSED_CATEGORY_NOT_EDITABLE],
        ),
        case(
            "category-in-gallery",
            json!({"remove_categories": ["Memes"]}),
            "refused",
            &[REFUSED_CATEGORY_NOT_EDITABLE],
        ),
        case(
            "unrelated-odd-categories-kept",
            json!({"remove_categories": ["Memes"]}),
            "changed",
            &[],
        ),
        case(
            "sort-keys-and-interwiki",
            json!({"classes": ["test:Kind"], "remove_categories": ["Memes"], "drop_generated": true}),
            "changed",
            &[],
        ),
        case(
            "mid-text-category",
            json!({"remove_categories": ["Memes"]}),
            "changed",
            &[],
        ),
        case(
            "two-on-one-line",
            json!({"remove_categories": ["Memes", "Topics"]}),
            "changed",
            &[],
        ),
        case(
            "one-of-two-on-one-line",
            json!({"remove_categories": ["Memes"]}),
            "changed",
            &[],
        ),
        case(
            "add-at-end",
            json!({"add_categories": ["Topics", "More_topics"]}),
            "changed",
            &[],
        ),
        case(
            "add-at-end-without-final-newline",
            json!({"add_categories": ["Category:Topics"]}),
            "changed",
            &[],
        ),
        case(
            "replace-every-category",
            json!({"remove_categories": ["Memes", "Old topics"], "add_categories": ["Topics"]}),
            "changed",
            &[],
        ),
        case(
            "trailing-whitespace",
            json!({"classes": ["test:Other"], "remove_categories": ["Memes"]}),
            "changed",
            &[],
        ),
        case(
            "last-line-without-newline",
            json!({"remove_categories": ["Memes"]}),
            "changed",
            &[],
        ),
        case(
            "multiple-class-calls",
            json!({"classes": ["test:Kind"]}),
            "refused",
            &[REFUSED_CLASS_STATEMENT_NOT_EDITABLE],
        ),
        case(
            "nested-class-call",
            json!({"classes": ["test:Other"]}),
            "refused",
            &[REFUSED_CLASS_STATEMENT_NOT_EDITABLE],
        ),
        case(
            "class-call-in-noinclude",
            json!({"classes": ["test:Other"]}),
            "refused",
            &[REFUSED_CLASS_STATEMENT_NOT_EDITABLE],
        ),
        case(
            "small-category-files-its-parent",
            json!({"classes": ["test:Unlisted"], "drop_generated": true}),
            "changed",
            &[],
        ),
    ]
}

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

/// Every fixture, in LF and in CRLF form: the expected status, refusal codes
/// and result, and the three properties of the proof.
#[test]
fn fixtures_give_the_expected_result_and_keep_every_other_byte() {
    let ontology = synthetic_ontology(&[]);
    let mut used = std::collections::BTreeSet::new();
    for case in cases() {
        used.insert(case.input);
        let plan = item(case.plan.clone());
        let lf = fixture(case.input);
        for (form, source, expected_result) in [
            ("lf", lf.clone(), expected(case.input)),
            (
                "crlf",
                crlf(&lf),
                expected(case.input).map(|text| crlf(&text)),
            ),
        ] {
            let label = format!("{} ({form}) with {}", case.input, case.plan);
            let outcome = run_and_check(&ontology, &plan, &source)
                .unwrap_or_else(|error| panic!("{label}: {error}"));
            let codes: Vec<&str> = outcome
                .refusals
                .iter()
                .map(|refusal| refusal.code)
                .collect();
            assert_eq!(codes, case.refusals, "{label}: {:#?}", outcome.refusals);
            let status = if !outcome.refusals.is_empty() {
                "refused"
            } else if outcome.result != source {
                "changed"
            } else {
                "unchanged"
            };
            assert_eq!(status, case.status, "{label}: {:#?}", outcome.edits);
            if status == "changed" {
                let expected_result =
                    expected_result.unwrap_or_else(|| panic!("{label}: no expected fixture"));
                assert_eq!(outcome.result, expected_result, "{label}");
            }
        }
    }
    for entry in fs::read_dir(fixtures()).expect("fixtures") {
        let name = entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .to_string();
        let stem = name
            .strip_suffix(".expected.wiki")
            .or_else(|| name.strip_suffix(".wiki"))
            .expect("fixture name");
        assert!(used.contains(stem), "fixture {name} has no case");
    }
}

#[test]
fn edits_name_their_kind_categories_and_exact_bytes() {
    let ontology = synthetic_ontology(&[]);
    let plan = item(json!({
        "classes": ["test:Kind"],
        "remove_categories": ["Memes"],
        "drop_generated": true
    }));
    let source = fixture("sort-keys-and-interwiki");
    let outcome = run_and_check(&ontology, &plan, &source).expect("proof");
    let edits: Vec<(&str, Vec<String>, &str, &str)> = outcome
        .edits
        .iter()
        .map(|edit| {
            (
                edit.kind,
                edit.categories.clone(),
                edit.old.as_str(),
                edit.new.as_str(),
            )
        })
        .collect();
    assert_eq!(
        edits,
        vec![
            (
                "class_statement_insert",
                vec![],
                "",
                "{{Instance of|test:Kind}}\n"
            ),
            (
                "category_remove",
                vec!["Memes".to_string()],
                "[[Category:Memes|Pi]]\n",
                ""
            ),
            (
                "category_drop_generated",
                vec!["Kinds".to_string()],
                "[[category:kinds]]\n",
                ""
            ),
        ]
    );
    assert_eq!(
        outcome.notes,
        vec![
            "kept the link to the category \"Kinds\": its sort key is not in the generated link"
                .to_string()
        ]
    );
    // The typing findings are the lint checks on the result.
    assert!(
        outcome
            .typing
            .checks_run
            .contains(&crate::ontology::typing::CHECK_CLASS_STATEMENTS)
    );
    assert!(outcome.typing.findings.is_empty(), "{:#?}", outcome.typing);
}

#[test]
fn refusals_name_the_context_and_the_typing_code() {
    let ontology = synthetic_ontology(&[]);
    let comment = retype_text(
        &ontology,
        &item(json!({"remove_categories": ["Memes"]})),
        &fixture("category-in-comment"),
    );
    assert_eq!(comment.refusals[0].context, Some("comment"));
    assert_eq!(comment.refusals[0].category.as_deref(), Some("Memes"));
    let gallery = retype_text(
        &ontology,
        &item(json!({"remove_categories": ["Memes"]})),
        &fixture("category-in-gallery"),
    );
    assert_eq!(gallery.refusals[0].context, Some("gallery"));
    let unknown = retype_text(
        &ontology,
        &item(json!({"classes": ["test:Nope"]})),
        &fixture("plain"),
    );
    assert_eq!(unknown.refusals[0].term.as_deref(), Some("test:Nope"));
    assert!(unknown.refusals[0].message.contains("test:Nope"));
    let renamed = retype_text(
        &ontology,
        &item(json!({"classes": ["test:Old"]})),
        &fixture("plain"),
    );
    assert!(renamed.refusals[0].message.contains("renamed to test:Kind"));
}

#[test]
fn plans_are_strict_and_name_each_title_once() {
    let plan = parse_plan(
        &json!({"schema": PLAN_SCHEMA, "items": [{"title": "Wojak", "classes": ["test:Kind"]}]})
            .to_string(),
    )
    .expect("plan");
    assert_eq!(plan.items[0].title, "Wojak");
    assert!(plan.items[0].remove_categories.is_empty());
    assert!(!plan.items[0].drop_generated);

    let wrong_schema = parse_plan(&json!({"schema": "other", "items": []}).to_string());
    assert!(wrong_schema.unwrap_err().to_string().contains(PLAN_SCHEMA));
    let unknown_field = parse_plan(
        &json!({"schema": PLAN_SCHEMA, "items": [{"title": "A", "colour": "red"}]}).to_string(),
    );
    assert!(unknown_field.is_err());
    let duplicate = parse_plan(
        &json!({"schema": PLAN_SCHEMA, "items": [{"title": "Wojak_x"}, {"title": "wojak x"}]})
            .to_string(),
    );
    assert!(
        duplicate
            .unwrap_err()
            .to_string()
            .contains("more than once")
    );
}

/// The full-mirror proof. It reads every page under the directory named by
/// `WIKITOOL_RETYPE_MIRROR` (a mirror's `wiki_content/Main`), runs the
/// synthetic plans on each, checks the three properties of every result, and
/// prints the counts. CI has no mirror, so the test is ignored; run it with
/// `cargo test -p wikitool_core retype_full_mirror -- --ignored --nocapture`.
#[test]
#[ignore = "needs WIKITOOL_RETYPE_MIRROR, a project mirror's wiki_content/Main"]
fn retype_full_mirror_keeps_every_byte_outside_the_edits() {
    let root =
        PathBuf::from(std::env::var("WIKITOOL_RETYPE_MIRROR").expect("set WIKITOOL_RETYPE_MIRROR"));
    let mut pages: Vec<(String, String)> = Vec::new();
    for entry in walkdir::WalkDir::new(&root).sort_by_file_name() {
        let entry = entry.expect("walk");
        let path = entry.path();
        if entry.file_type().is_file() && path.extension().is_some_and(|ext| ext == "wiki") {
            let text = fs::read_to_string(path).expect("read page");
            pages.push((
                path.strip_prefix(&root).unwrap().display().to_string(),
                text,
            ));
        }
    }
    assert!(!pages.is_empty(), "no pages under {}", root.display());

    // One synthetic class per category the mirror uses, so drop_generated has
    // a class whose category each page links.
    let mut categories = std::collections::BTreeSet::new();
    for (_, text) in &pages {
        for name in page_categories(text) {
            categories.insert(name);
        }
    }
    let extra: Vec<(String, String)> = categories
        .iter()
        .enumerate()
        .map(|(index, name)| (format!("test:C{index}"), name.clone()))
        .collect();
    let class_for: BTreeMap<String, String> = extra
        .iter()
        .map(|(id, name)| (name.clone(), id.clone()))
        .collect();
    let ontology = synthetic_ontology(&extra);

    let mut plans_run = 0usize;
    let mut changed = 0usize;
    let mut edits = 0usize;
    let mut edits_by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut plans_by_name: BTreeMap<&str, usize> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    let mut odd: BTreeMap<&str, usize> = BTreeMap::new();
    for (path, source) in &pages {
        for (marker, needle) in [
            ("comment", "<!--"),
            ("nowiki", "<nowiki"),
            ("includeonly", "<includeonly"),
            ("noinclude", "<noinclude"),
            ("gallery", "<gallery"),
            ("crlf", "\r\n"),
            ("trailing_whitespace", " \n"),
        ] {
            if source.contains(needle) {
                *odd.entry(marker).or_default() += 1;
            }
        }
        if let Ok(document) = parse_profiled(ProfiledParseInput {
            source: &wikitext_profile(source),
            wikitext: source,
        }) {
            let links = category_links(&document);
            if links.iter().any(|link| link.sort_key) {
                *odd.entry("sort_key").or_default() += 1;
            }
            if links.iter().any(|link| link.context.is_some()) {
                *odd.entry("category_not_editable").or_default() += 1;
            }
            if document.nodes().iter().any(|node| {
                matches!(node.kind(), NodeKind::InternalLink(link) if link.kind == LinkKind::Other)
            }) {
                *odd.entry("interwiki_or_other_namespace_link").or_default() += 1;
            }
        }
        let mut plans: Vec<(&str, RetypePlanItem)> = vec![
            ("add_class", item(json!({"classes": ["test:Kind"]}))),
            (
                "add_category",
                item(json!({"add_categories": ["Retype synthetic category"]})),
            ),
            (
                "remove_absent",
                item(json!({"remove_categories": ["Retype synthetic absent"]})),
            ),
        ];
        let names = page_categories(source);
        for name in &names {
            plans.push((
                "remove_category",
                item(json!({"remove_categories": [name]})),
            ));
        }
        if let Some(first) = names.first() {
            plans.push((
                "drop_generated",
                item(json!({"classes": [class_for[first]], "drop_generated": true})),
            ));
        }
        let mut chained: Option<String> = None;
        for (name, plan) in plans {
            plans_run += 1;
            *plans_by_name.entry(name).or_default() += 1;
            match run_and_check(&ontology, &plan, source) {
                Ok(outcome) => {
                    if name == "add_class" && outcome.result != *source {
                        chained = Some(outcome.result.clone());
                    }
                    tally(
                        &outcome,
                        source,
                        &mut changed,
                        &mut edits,
                        &mut edits_by_kind,
                        &mut refusals,
                    );
                }
                Err(error) => failures.push(format!("{path} {name}: {error}")),
            }
        }
        // Replace the class statement the first plan added.
        if let Some(with_call) = chained {
            plans_run += 1;
            *plans_by_name.entry("replace_class").or_default() += 1;
            let plan = item(json!({
                "classes": ["test:Other", "test:Kind"],
                "relations": {"movement": ["Retype synthetic movement"]},
                "wikidata": "Q1"
            }));
            match run_and_check(&ontology, &plan, &with_call) {
                Ok(outcome) => {
                    if outcome.result == with_call && outcome.refusals.is_empty() {
                        failures.push(format!("{path} replace_class: no change"));
                    }
                    tally(
                        &outcome,
                        &with_call,
                        &mut changed,
                        &mut edits,
                        &mut edits_by_kind,
                        &mut refusals,
                    );
                }
                Err(error) => failures.push(format!("{path} replace_class: {error}")),
            }
        }
    }
    println!("retype full-mirror proof over {}", root.display());
    println!("pages: {}", pages.len());
    println!("pages with: {odd:?}");
    println!("plans run: {plans_run} {plans_by_name:?}");
    println!("changed results: {changed}");
    println!("edits: {edits} {edits_by_kind:?}");
    println!(
        "refusals: {} {refusals:?}",
        refusals.values().sum::<usize>()
    );
    println!("failures: {}", failures.len());
    for failure in failures.iter().take(20) {
        println!("  {failure}");
    }
    assert!(failures.is_empty(), "{} failures", failures.len());
}

fn tally(
    outcome: &PageRetype,
    source: &str,
    changed: &mut usize,
    edits: &mut usize,
    edits_by_kind: &mut BTreeMap<&'static str, usize>,
    refusals: &mut BTreeMap<String, usize>,
) {
    if outcome.result != source {
        *changed += 1;
    }
    *edits += outcome.edits.len();
    for edit in &outcome.edits {
        *edits_by_kind.entry(edit.kind).or_default() += 1;
    }
    for refusal in &outcome.refusals {
        let key = match refusal.context {
            Some(context) => format!("{}:{context}", refusal.code),
            None => refusal.code.to_string(),
        };
        *refusals.entry(key).or_default() += 1;
    }
}

/// The distinct categories a page's links name, editable or not, in order.
fn page_categories(text: &str) -> Vec<String> {
    let profile = wikitext_profile(text);
    let Ok(document) = parse_profiled(ProfiledParseInput {
        source: &profile,
        wikitext: text,
    }) else {
        return Vec::new();
    };
    let mut seen = std::collections::BTreeSet::new();
    category_links(&document)
        .into_iter()
        .map(|link| link.name)
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

// Review 1 regressions. Each test fails without the fix it names.

fn codes(outcome: &PageRetype) -> Vec<&'static str> {
    outcome
        .refusals
        .iter()
        .map(|refusal| refusal.code)
        .collect()
}

/// Finding 3: a field value or Wikidata id that holds markup is refused, and
/// the result check refuses a class statement that would add a category.
#[test]
fn values_that_hold_markup_are_refused() {
    let ontology = synthetic_ontology(&[]);
    let source = fixture("plain");
    for plan in [
        json!({"classes": ["test:Kind"], "relations": {"movement": ["x <!-- y"]}}),
        json!({"classes": ["test:Kind"], "relations": {"movement": ["[[Category:Z]]"]}}),
        json!({"classes": ["test:Kind"], "relations": {"movement": ["a __NOINDEX__"]}}),
        json!({"classes": ["test:Kind"], "relations": {"movement": ["<ref>x</ref>"]}}),
        json!({"classes": ["test:Kind"], "wikidata": "Q1 [[Category:Z]]"}),
        json!({"classes": ["test:Kind"], "wikidata": "Q1<!--"}),
        json!({"classes": ["test:Kind"], "wikidata": "wikidata"}),
    ] {
        let outcome = run_and_check(&ontology, &item(plan.clone()), &source)
            .unwrap_or_else(|error| panic!("{plan}: {error}"));
        assert_eq!(codes(&outcome), vec![REFUSED_INVALID_PLAN], "{plan}");
        assert_eq!(outcome.result, source, "{plan}");
    }
}

#[test]
fn the_result_check_refuses_a_class_statement_that_adds_a_category() {
    let source = "Text\n";
    let new = "{{Instance of|test:Kind|movement=[[Category:Z]]}}\n";
    let result = format!("{new}{source}");
    let edits = vec![RetypeEdit {
        kind: "class_statement_insert",
        categories: Vec::new(),
        start: 0,
        end: 0,
        result_start: 0,
        result_end: new.len(),
        old: String::new(),
        new: new.to_string(),
    }];
    let error = guard::check_result(source, &result, &edits, "Template:Instance of")
        .expect_err("an added category link");
    assert!(error.contains("new link"), "{error}");
    // A plain class statement passes.
    let new = "{{Instance of|test:Kind}}\n";
    let mut edits = edits;
    edits[0].new = new.to_string();
    edits[0].result_end = new.len();
    guard::check_result(
        source,
        &format!("{new}{source}"),
        &edits,
        "Template:Instance of",
    )
    .expect("a plain class statement");
}

/// Finding 5: the generated categories are the ones `Module:Infobox`'s
/// `instanceOf` files: its walk stops at a `subClassOf` list and reads
/// superclasses only from classes, properties and relations.
#[test]
fn generated_categories_follow_the_module_walk() {
    let ontology = synthetic_ontology_edited(&[], |shared, _| {
        shared["classes"]["test:Multi"] =
            json!({"label": "Multi", "subClassOf": ["test:Kind", "test:Other"]});
        shared["classes"]["test:ViaFamily"] =
            json!({"label": "Via family", "subClassOf": "test:Fam"});
        shared["families"]["test:Fam"] = json!({"label": "Fam", "subClassOf": "test:Kind"});
        shared["classes"]["test:ViaRename"] =
            json!({"label": "Via rename", "subClassOf": "test:Old"});
    });
    let source = "'''Page''' text.\n\n[[Category:Kinds]]\n[[Category:Others]]\n";
    for (class, note) in [
        ("test:Multi", "lists several classes in subClassOf"),
        (
            "test:ViaFamily",
            "neither it nor a superclass has a category",
        ),
        ("test:ViaRename", "which the ontology renamed to test:Kind"),
    ] {
        let plan = item(json!({"classes": [class], "drop_generated": true}));
        let outcome = run_and_check(&ontology, &plan, source).expect("proof");
        assert!(
            outcome.refusals.is_empty(),
            "{class}: {:#?}",
            outcome.refusals
        );
        assert_eq!(
            outcome.result,
            format!("{{{{Instance of|{class}}}}}\n{source}"),
            "{class}: the module files the page in no category, so no link is dropped"
        );
        assert!(
            outcome.notes.iter().any(|text| text.contains(note)),
            "{class}: {:?}",
            outcome.notes
        );
    }
    // A single superclass with a category is followed.
    let plan = item(json!({"classes": ["test:Unlisted"], "drop_generated": true}));
    let (generated, _) = generated_categories(&ontology, &["test:Unlisted".to_string()]);
    assert_eq!(
        generated,
        vec!["Small kinds".to_string(), "Kinds".to_string()]
    );
    run_and_check(&ontology, &plan, source).expect("proof");
}

/// Finding 6: on a page whose typed infobox states the class, the planned
/// classes are still checked, and a plan that adds a category drop_generated
/// would remove is still refused.
#[test]
fn a_typed_infobox_page_still_checks_the_planned_classes() {
    let ontology = synthetic_ontology_edited(&[], |shared, _| {
        shared["renames"]["test:Human"] = json!("test:Person");
    });
    let source = fixture("typed-infobox-states-the-class");
    let renamed = run_and_check(
        &ontology,
        &item(json!({"classes": ["test:Human"]})),
        &source,
    )
    .expect("proof");
    assert_eq!(codes(&renamed), vec![RENAMED_TERM]);
    assert!(
        renamed.refusals[0]
            .message
            .contains("renamed to test:Person")
    );
    let contradiction = run_and_check(
        &ontology,
        &item(json!({"classes": ["test:Person"], "add_categories": ["People"], "drop_generated": true})),
        &source,
    )
    .expect("proof");
    assert_eq!(codes(&contradiction), vec![REFUSED_INVALID_PLAN]);
    // Without the contradiction the page is left as it is, with a note.
    let kept = run_and_check(
        &ontology,
        &item(json!({"classes": ["test:Person"], "drop_generated": true})),
        &source,
    )
    .expect("proof");
    assert!(kept.refusals.is_empty());
    assert_eq!(kept.result, source);
    assert!(
        kept.notes
            .iter()
            .any(|text| text.contains("drop_generated removed nothing"))
    );
}

/// Finding 7: a gallery whose opening tag holds an apostrophe still counts.
#[test]
fn a_gallery_with_an_apostrophe_in_its_caption_is_not_edited() {
    let ontology = synthetic_ontology(&[]);
    let source =
        "<gallery caption=Bob's>\nFile:X.png|[[Category:Memes]]\n</gallery>\n[[Category:Memes]]\n";
    let outcome = run_and_check(
        &ontology,
        &item(json!({"remove_categories": ["Memes"]})),
        source,
    )
    .expect("proof");
    assert_eq!(codes(&outcome), vec![REFUSED_CATEGORY_NOT_EDITABLE]);
    assert_eq!(outcome.refusals[0].context, Some("gallery"));
}

/// Finding 8: new text never lands inside a reference or an unclosed region.
#[test]
fn insertions_stay_out_of_references() {
    let ontology = synthetic_ontology(&[]);
    // The short description's line ends inside a reference, so the call goes
    // right after the short description instead.
    let source = "{{Short description|x}}<ref>a\nb</ref>\nText.\n";
    let outcome =
        run_and_check(&ontology, &item(json!({"classes": ["test:Kind"]})), source).expect("proof");
    assert!(outcome.refusals.is_empty(), "{:#?}", outcome.refusals);
    assert_eq!(
        outcome.result,
        "{{Short description|x}}\n{{Instance of|test:Kind}}<ref>a\nb</ref>\nText.\n"
    );
    // A page that ends inside an unclosed reference gets no new link there.
    let source = "Text.<ref>Note\n";
    let outcome = run_and_check(
        &ontology,
        &item(json!({"add_categories": ["Topics"]})),
        source,
    )
    .expect("proof");
    assert_eq!(codes(&outcome), vec![REFUSED_CATEGORY_NOT_EDITABLE]);
    assert_eq!(outcome.refusals[0].context, Some("ref"));
    // A link inside math is not edited.
    let source = "<math>[[Category:Memes]]</math>\n[[Category:Memes]]\n";
    let outcome = run_and_check(
        &ontology,
        &item(json!({"remove_categories": ["Memes"]})),
        source,
    )
    .expect("proof");
    assert_eq!(codes(&outcome), vec![REFUSED_CATEGORY_NOT_EDITABLE]);
    assert_eq!(outcome.refusals[0].context, Some("math"));
}

/// Finding 9: a comment that does not parse as wikitext still shows its links.
#[test]
fn a_link_in_a_comment_that_does_not_parse_is_seen() {
    let ontology = synthetic_ontology(&[]);
    let source = "<!-- {{x [[Category:Memes]] -->\nText.\n[[Category:Memes]]\n";
    let outcome = run_and_check(
        &ontology,
        &item(json!({"remove_categories": ["Memes"]})),
        source,
    )
    .expect("proof");
    assert_eq!(codes(&outcome), vec![REFUSED_CATEGORY_NOT_EDITABLE]);
    assert_eq!(outcome.refusals[0].context, Some("comment"));
}

/// Nits: spacing after a mid-line removal, a byte order mark, numbered class
/// slots, and the line break of new lines.
#[test]
fn small_edits_keep_the_page_tidy() {
    let ontology = synthetic_ontology(&[]);
    let remove = item(json!({"remove_categories": ["Memes"]}));
    for (source, expected) in [
        ("Text [[Category:Memes]] more.\n", "Text more.\n"),
        ("Text [[Category:Memes]]\n", "Text\n"),
        ("\u{feff}[[Category:Memes]]\nText\n", "\u{feff}Text\n"),
    ] {
        let outcome = run_and_check(&ontology, &remove, source).expect("proof");
        assert_eq!(outcome.result, expected, "{source:?}");
    }
    for (source, plan, expected) in [
        (
            "{{Instance of|1=test:Other}}\nText\n",
            json!({"classes": ["test:Kind"]}),
            "{{Instance of|1=test:Kind}}\nText\n",
        ),
        (
            "{{Instance of|1=test:Kind}}\nText\n",
            json!({"classes": ["test:Kind", "test:Other"]}),
            "{{Instance of|1=test:Kind|2=test:Other}}\nText\n",
        ),
        (
            "{{Instance of|1=test:Kind}}\nText\n",
            json!({"classes": ["test:Kind"]}),
            "{{Instance of|1=test:Kind}}\nText\n",
        ),
    ] {
        let outcome = run_and_check(&ontology, &item(plan.clone()), source).expect("proof");
        assert!(
            outcome.refusals.is_empty(),
            "{plan}: {:#?}",
            outcome.refusals
        );
        assert_eq!(outcome.result, expected, "{plan}");
    }
    for source in [
        "{{Instance of|2=test:Other|1=test:Kind}}\n",
        "{{Instance of|test:Other|1=test:Kind}}\n",
    ] {
        let outcome = run_and_check(&ontology, &item(json!({"classes": ["test:Kind"]})), source)
            .expect("proof");
        assert_eq!(
            codes(&outcome),
            vec![REFUSED_CLASS_STATEMENT_NOT_EDITABLE],
            "{source}"
        );
    }
    // One stray CRLF does not make the new lines CRLF.
    let outcome = run_and_check(
        &ontology,
        &item(json!({"classes": ["test:Kind"], "add_categories": ["Topics"]})),
        "A\r\nB\nC\n[[Category:Memes]]\n",
    )
    .expect("proof");
    assert_eq!(
        outcome.result,
        "{{Instance of|test:Kind}}\nA\r\nB\nC\n[[Category:Memes]]\n[[Category:Topics]]\n"
    );
}

/// Nit: whatever the plan does, a page keeps its final line break, or its
/// lack of one.
#[test]
fn every_result_keeps_the_final_line_break_state() {
    let ontology = synthetic_ontology(&[]);
    let sources = [
        "[[Category:Memes]]",
        "Text\n[[Category:Memes]]",
        "[[Category:Memes]]\nText",
        "Text [[Category:Memes]]",
        "{{SHORTDESC:x}}\nText\n[[Category:Memes]] [[Category:Topics]]",
        "{{SHORTDESC:x}}",
        "{{Infobox thing}}\nText [[Category:Memes]]",
        "Text\n\n[[Category:Memes]]\n\n",
        "Text",
        "",
    ];
    let plans = [
        json!({"remove_categories": ["Memes"]}),
        json!({"add_categories": ["Added"]}),
        json!({"classes": ["test:Kind"]}),
        json!({"classes": ["test:Kind"], "remove_categories": ["Memes"], "add_categories": ["Added"]}),
        json!({"classes": ["test:Kind"], "drop_generated": true}),
        json!({"classes": ["test:Kind"], "remove_categories": ["Memes"]}),
    ];
    for base in sources {
        for source in [
            base.to_string(),
            format!("{base}\n"),
            crlf(base),
            crlf(&format!("{base}\n")),
        ] {
            for plan in &plans {
                let outcome = run_and_check(&ontology, &item(plan.clone()), &source)
                    .unwrap_or_else(|error| panic!("{source:?} {plan}: {error}"));
                if outcome.result.is_empty() || !outcome.refusals.is_empty() {
                    continue;
                }
                assert_eq!(
                    outcome.result.ends_with('\n'),
                    source.ends_with('\n'),
                    "{source:?} {plan}: {:?}",
                    outcome.result
                );
            }
        }
    }
}

// Findings 1, 2 and 4: where pages are read and written.

fn project_paths(root: &Path) -> crate::runtime::ResolvedPaths {
    use crate::runtime::{ResolvedPaths, ValueSource};
    ResolvedPaths {
        wiki_content_dir: root.join("wiki_content"),
        templates_dir: root.join("templates"),
        state_dir: root.join(".wikitool"),
        data_dir: root.join(".wikitool/data"),
        db_path: root.join(".wikitool/data/wikitool.db"),
        config_path: root.join(".wikitool/config.toml"),
        parser_config_path: root.join(".wikitool/parser.toml"),
        project_root: root.to_path_buf(),
        root_source: ValueSource::Flag,
        data_source: ValueSource::Default,
        config_source: ValueSource::Default,
    }
}

fn put(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, text).expect("write");
}

fn located(
    files: &pages::PageFiles<'_>,
    title: &str,
) -> Result<(String, &'static str, String), (&'static str, String)> {
    match locate_title(files, title).expect("locate") {
        Located::Page {
            source_path,
            source_kind,
            draft_path,
        } => Ok((source_path, source_kind, draft_path)),
        Located::Refused(refusal) => Err((refusal.code, refusal.message)),
    }
}

#[test]
fn titles_are_located_with_the_configured_namespaces_and_exact_file_names() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().canonicalize().expect("root");
    let paths = project_paths(&root);
    put(&root, "wiki_content/Main/Foo.wiki", "Foo\n");
    put(&root, "wiki_content/Goldenlight/Foo.wiki", "Golden\n");
    put(&root, "wiki_content/Main/Foo_bar.wiki", "Foo bar\n");
    put(&root, "wiki_content/Main/Miya_Notes.wiki", "Mirror\n");
    put(&root, ".wikitool/drafts/Miya Notes.wiki", "Human edits\n");
    put(&root, "wiki_content/Main/Twin_page.wiki", "Twin\n");
    put(&root, ".wikitool/drafts/twin_Page.wiki", "Other title\n");
    put(&root, "wiki_content/Main/Two.wiki", "Two\n");
    put(&root, ".wikitool/drafts/Two.wiki", "a\n");
    put(&root, ".wikitool/drafts/nested/Two.wiki", "b\n");
    let files = pages::PageFiles::load(&paths).expect("files");

    assert_eq!(
        located(&files, "Foo"),
        Ok((
            "wiki_content/Main/Foo.wiki".to_string(),
            "mirror",
            ".wikitool/drafts/Foo.wiki".to_string()
        ))
    );
    // A custom namespace is not the article namespace, and its page never
    // reads or writes the draft of the article with the same name.
    assert_eq!(
        located(&files, "Goldenlight:Foo").unwrap_err().0,
        REFUSED_NOT_AN_ARTICLE
    );
    assert_eq!(
        located(&files, "Goldenlight talk:Foo").unwrap_err().0,
        REFUSED_NOT_AN_ARTICLE
    );
    // A title never reads another title's file, even on a case-insensitive
    // file system.
    assert_eq!(
        located(&files, "Foo Bar").unwrap_err().0,
        REFUSED_PAGE_MISSING
    );
    // A draft with a free name is the page's source and target.
    assert_eq!(
        located(&files, "Miya Notes"),
        Ok((
            ".wikitool/drafts/Miya Notes.wiki".to_string(),
            "draft",
            ".wikitool/drafts/Miya Notes.wiki".to_string()
        ))
    );
    // A new draft whose name another draft differs from only in case.
    assert_eq!(
        located(&files, "Twin page").unwrap_err().0,
        REFUSED_DUPLICATE_TARGET
    );
    // Two drafts for one title.
    assert_eq!(
        located(&files, "Two").unwrap_err().0,
        REFUSED_AMBIGUOUS_DRAFT
    );
    // A title that would name a path outside the drafts.
    assert_eq!(
        located(&files, r"..\..\wiki_content\Main\Foo")
            .unwrap_err()
            .0,
        REFUSED_INVALID_TITLE
    );
    assert!(
        files
            .checked_draft(".wikitool/drafts/../../wiki_content/Main/Foo.wiki")
            .is_err()
    );
    assert!(files.checked_draft("wiki_content/Main/Foo.wiki").is_err());

    // Two titles that write one draft, compared without case, are both refused.
    let mut selected = vec![
        (
            "Foo bar".to_string(),
            Located::Page {
                source_path: "wiki_content/Main/Foo_bar.wiki".to_string(),
                source_kind: "mirror",
                draft_path: ".wikitool/drafts/Foo_bar.wiki".to_string(),
            },
        ),
        (
            "Foo Bar".to_string(),
            Located::Page {
                source_path: "wiki_content/Main/Foo_Bar.wiki".to_string(),
                source_kind: "mirror",
                draft_path: ".wikitool/drafts/Foo_Bar.wiki".to_string(),
            },
        ),
    ];
    refuse_shared_targets(&mut selected);
    for (title, located) in &selected {
        assert!(
            matches!(located, Located::Refused(refusal) if refusal.code == REFUSED_DUPLICATE_TARGET),
            "{title}"
        );
    }

    // A PATH is retyped exactly: a draft in place, and a mirror page only
    // while its title has no draft.
    let (title, at) = locate_file(
        &paths,
        &files,
        Path::new(".wikitool/drafts/Miya Notes.wiki"),
    )
    .expect("draft path");
    assert_eq!(title, "Miya Notes");
    assert!(
        matches!(at, Located::Page { draft_path, .. } if draft_path == ".wikitool/drafts/Miya Notes.wiki")
    );
    let (_, at) = locate_file(
        &paths,
        &files,
        Path::new("wiki_content/Main/Miya_Notes.wiki"),
    )
    .expect("mirror path");
    assert!(matches!(at, Located::Refused(refusal) if refusal.code == REFUSED_DRAFT_EXISTS));
    let (title, at) = locate_file(
        &paths,
        &files,
        Path::new("wiki_content/Goldenlight/Foo.wiki"),
    )
    .expect("custom namespace path");
    assert_eq!(title, "Goldenlight:Foo");
    assert!(matches!(at, Located::Refused(refusal) if refusal.code == REFUSED_NOT_AN_ARTICLE));
    assert!(locate_file(&paths, &files, Path::new("plans/x.wiki")).is_err());
}

// Review 2 regressions. Each test fails without the fix it names.

fn titled(title: &str, plan: Value) -> RetypePlanItem {
    let mut plan = plan;
    plan["title"] = json!(title);
    serde_json::from_value(plan).expect("plan item")
}

/// Finding 1: in title mode, a mirror page is never retyped over a file at
/// its draft path, even one the draft index files under another title.
#[test]
fn a_mirror_page_is_not_retyped_over_a_file_at_its_draft_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().canonicalize().expect("root");
    let paths = project_paths(&root);
    put(&root, "wiki_content/Main/Foo--Bar.wiki", "Mirror\n");
    put(&root, ".wikitool/drafts/Foo--Bar.wiki", "Human edits\n");
    put(&root, "wiki_content/Main/Foo____Bar.wiki", "Mirror\n");
    put(&root, ".wikitool/drafts/Foo____Bar.wiki", "Human edits\n");
    put(&root, "wiki_content/Main/Plain.wiki", "Mirror\n");
    let files = pages::PageFiles::load(&paths).expect("files");
    // `Foo--Bar.wiki` reads back as `Foo:Bar`, and `Foo____Bar.wiki` as
    // `Foo/ Bar`, so the index files neither draft under its own title.
    assert!(files.drafts_for("Foo--Bar").is_empty());
    for title in ["Foo--Bar", "Foo /Bar"] {
        assert_eq!(
            located(&files, title).unwrap_err().0,
            REFUSED_DRAFT_EXISTS,
            "{title}"
        );
    }
    assert!(located(&files, "Plain").is_ok());
}

/// Test gap of review 1: the exact-name and case-twin checks, on any file
/// system.
#[test]
fn file_names_are_compared_exactly() {
    let listing = vec!["Foo_bar.wiki".to_string(), "Other.wiki".to_string()];
    assert!(pages::exact_name_in(&listing, "Foo_bar.wiki"));
    assert!(!pages::exact_name_in(&listing, "Foo_Bar.wiki"));
    assert!(!pages::exact_name_in(&listing, "foo_bar.wiki"));
    assert_eq!(
        pages::case_twin_in(&listing, "Foo_Bar.wiki").map(String::as_str),
        Some("Foo_bar.wiki")
    );
    assert_eq!(pages::case_twin_in(&listing, "Foo_bar.wiki"), None);
    assert_eq!(pages::case_twin_in(&listing, "Third.wiki"), None);
}

/// Finding 2: `Module:Infobox` files a translation subpage in no category,
/// so drop_generated removes nothing there.
#[test]
fn drop_generated_removes_nothing_on_a_translation_subpage() {
    let ontology = synthetic_ontology(&[]);
    let source = "'''Page''' text.\n\n[[Category:Kinds]]\n";
    let plan = json!({"classes": ["test:Kind"], "drop_generated": true});
    let subpage =
        run_and_check(&ontology, &titled("Wojak/ja", plan.clone()), source).expect("proof");
    assert!(subpage.refusals.is_empty(), "{:#?}", subpage.refusals);
    assert_eq!(
        subpage.result,
        format!("{{{{Instance of|test:Kind}}}}\n{source}")
    );
    assert!(
        subpage
            .notes
            .iter()
            .any(|text| text.contains("translation subpage (/ja)")),
        "{:?}",
        subpage.notes
    );
    // An article, and a subpage that is not a translation, still drop it.
    for title in ["Wojak", "Wojak/Gallery"] {
        let outcome =
            run_and_check(&ontology, &titled(title, plan.clone()), source).expect("proof");
        assert!(
            !outcome.result.contains("[[Category:Kinds]]"),
            "{title}: {:?}",
            outcome.result
        );
    }
}

/// Finding 3: a planned field the display module does not define, which
/// files the page in a tracking category, is refused; a finding the page
/// already has does not block.
#[test]
fn a_result_with_a_new_typing_finding_is_refused() {
    let ontology = synthetic_ontology(&[]);
    for field in ["movment", "Movement"] {
        let plan = json!({"classes": ["test:Kind"], "relations": {field: ["New Net Art"]}});
        let outcome = run_and_check(&ontology, &item(plan), &fixture("plain")).expect("proof");
        assert_eq!(
            codes(&outcome),
            vec![crate::ontology::typing::UNKNOWN_FIELD],
            "{field}"
        );
        assert_eq!(outcome.refusals[0].term.as_deref(), Some(field));
    }
    let source = "{{Instance of|test:Kind|movment=x}}\nText\n";
    let outcome =
        run_and_check(&ontology, &item(json!({"classes": ["test:Other"]})), source).expect("proof");
    assert!(outcome.refusals.is_empty(), "{:#?}", outcome.refusals);
    assert_eq!(
        outcome.result,
        "{{Instance of|test:Other|movment=x}}\nText\n"
    );
}

/// Finding 5: a category name MediaWiki decodes (`%`, `#`, `&`) cannot be
/// read from the text, so a plan that edits categories on that page is
/// refused, and a plan may not name one.
#[test]
fn category_names_that_mediawiki_decodes_are_unreadable() {
    let ontology = synthetic_ontology(&[]);
    for variant in [
        "[[Category:Foo%20bar]]",
        "[[Category:Foo bar#x]]",
        "[[Category:Foo &amp; bar]]",
    ] {
        let source = format!("Text\n[[Category:Foo bar]]\n{variant}\n");
        let outcome = run_and_check(
            &ontology,
            &item(json!({"remove_categories": ["Foo bar"]})),
            &source,
        )
        .expect("proof");
        assert_eq!(
            codes(&outcome),
            vec![REFUSED_CATEGORY_NOT_EDITABLE],
            "{variant}"
        );
        assert_eq!(outcome.refusals[0].context, Some("unreadable"), "{variant}");
    }
    for name in ["Foo%20bar", "Tom & Jerry"] {
        let outcome = run_and_check(
            &ontology,
            &item(json!({"add_categories": [name]})),
            &fixture("plain"),
        )
        .expect("proof");
        assert_eq!(codes(&outcome), vec![REFUSED_INVALID_PLAN], "{name}");
    }
}

/// Nit: drop_generated with nothing to do says so.
#[test]
fn drop_generated_with_no_class_statement_notes_it() {
    let ontology = synthetic_ontology(&[]);
    let outcome = run_and_check(
        &ontology,
        &item(json!({"drop_generated": true})),
        &fixture("plain"),
    )
    .expect("proof");
    assert!(outcome.refusals.is_empty());
    assert!(
        outcome
            .notes
            .iter()
            .any(|text| text
                .contains("drop_generated removed nothing: the page has no class statement")),
        "{:?}",
        outcome.notes
    );
}
