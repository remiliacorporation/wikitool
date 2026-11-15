use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::proof::{run_and_check, synthetic_ontology};
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
