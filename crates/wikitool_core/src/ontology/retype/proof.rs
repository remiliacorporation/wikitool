//! The three properties every retype result must have, and a synthetic
//! ontology to run plans against. The fixture tests and the mirror test both
//! use them.
//!
//! 1. The result is the source with the reported spans swapped back.
//! 2. A second run of the same plan on the result changes nothing.
//! 3. Re-parsing the result gives the same nodes and tag regions outside the
//!    edited spans.

use std::collections::BTreeSet;

use mediawiki_wikitext::{Document, NodeKind, ProfiledParseInput, parse_profiled};
use serde_json::{Value, json};

use crate::ontology::typing::{ModuleStatus, ProjectOntology};
use crate::ontology::values::wikitext_profile;
use crate::site::adapter::model::OntologyRules;

use super::{PageRetype, RetypePlanItem, retype_text};

const RULES: &str = r#"
product = "test-wiki"
source = "https://ontology.example/"
local_prefix = "test"
local_namespace = "https://wiki.example/ns/"
class_template = "Template:Instance of"
typed_infobox_template = "Template:Infobox"

[infobox_types]
"Template:Infobox person" = "person"
"#;

/// A small ontology: `test:Kind` and its subclasses, a renamed id, a person
/// kind of page, and relation fields. `extra` adds classes, each filing its
/// articles in the named category.
pub(crate) fn synthetic_ontology(extra: &[(String, String)]) -> ProjectOntology {
    let rules: OntologyRules = toml::from_str(RULES).expect("rules");
    let mut classes = json!({
        "test:Thing": { "label": "Thing" },
        "test:Kind": { "label": "Kind", "subClassOf": "test:Thing" },
        "test:Small": { "label": "Small kind", "subClassOf": "test:Kind" },
        "test:Unlisted": { "label": "Unlisted kind", "subClassOf": "test:Small" },
        "test:Other": { "label": "Other" },
        "test:Person": { "label": "Person" },
    });
    let mut display_classes = json!({
        "test:Thing": { "category": "Things" },
        "test:Kind": { "category": "Kinds" },
        "test:Small": { "category": "Small kinds", "small": true },
        "test:Other": { "category": "Others" },
        "test:Person": { "category": "People" },
    });
    for (id, category) in extra {
        classes[id] = json!({ "label": id, "subClassOf": "test:Thing" });
        display_classes[id] = json!({ "category": category });
    }
    let shared = json!({
        "classes": classes,
        "families": {},
        "properties": {
            "wdt:P135": { "label": "movement", "valueType": "canonical-object", "cardinality": "many" }
        },
        "relations": {
            "test:memberOf": { "label": "Member of", "domainClasses": ["test:Thing"] }
        },
        "schemes": {},
        "mappings": {},
        "renames": { "test:Old": "test:Kind" },
        "external": {},
        "source": { "commit": "fixture-commit", "product": "test-wiki" }
    });
    let display = json!({
        "fields": {
            "movement": { "property": "wdt:P135", "label": "Movement", "category": "%s" },
            "member_of": { "property": "test:memberOf", "label": "Member of", "category": "%s" }
        },
        "types": {
            "person": { "class": "test:Person", "sections": [] }
        },
        "classes": display_classes,
        "existingCategories": {}
    });
    let loaded = |title| ModuleStatus {
        title,
        relative_path: String::new(),
        status: "loaded",
        error: None,
    };
    ProjectOntology::new(
        rules,
        shared,
        (
            loaded("Module:Ontology/local"),
            Value::Object(Default::default()),
        ),
        (loaded("Module:Ontology/display"), display),
    )
}

/// Run one plan and check the three properties. Returns the outcome.
pub(crate) fn run_and_check(
    ontology: &ProjectOntology,
    item: &RetypePlanItem,
    source: &str,
) -> Result<PageRetype, String> {
    let outcome = retype_text(ontology, item, source);
    check_outcome(ontology, item, source, &outcome)?;
    Ok(outcome)
}

pub(crate) fn check_outcome(
    ontology: &ProjectOntology,
    item: &RetypePlanItem,
    source: &str,
    outcome: &PageRetype,
) -> Result<(), String> {
    if !outcome.refusals.is_empty() && (outcome.result != source || !outcome.edits.is_empty()) {
        return Err("a refused page has a result or edits".to_string());
    }

    // 1. The spans swapped back give the source.
    let mut previous_end = 0;
    for edit in &outcome.edits {
        if edit.start < previous_end || edit.end < edit.start {
            return Err(format!("edits overlap or run backwards at {}", edit.start));
        }
        previous_end = edit.end;
        if source.get(edit.start..edit.end) != Some(edit.old.as_str()) {
            return Err(format!(
                "edit at {} does not report the source bytes",
                edit.start
            ));
        }
        if outcome.result.get(edit.result_start..edit.result_end) != Some(edit.new.as_str()) {
            return Err(format!(
                "edit at {} does not report the result bytes",
                edit.result_start
            ));
        }
    }
    let mut restored = outcome.result.clone();
    for edit in outcome.edits.iter().rev() {
        restored.replace_range(edit.result_start..edit.result_end, &edit.old);
    }
    if restored != source {
        return Err("swapping the reported spans back does not give the source".to_string());
    }

    // 2. A second run changes nothing.
    let second = retype_text(ontology, item, &outcome.result);
    if second.result != outcome.result {
        return Err(format!(
            "a second run changed the result: {:?}",
            second.edits
        ));
    }

    // 3. The same nodes outside the edits.
    if outcome.edits.is_empty() {
        return Ok(());
    }
    let before = parse(source).map_err(|error| format!("source does not parse: {error}"))?;
    let after =
        parse(&outcome.result).map_err(|error| format!("result does not parse: {error}"))?;
    let source_spans: Vec<(usize, usize)> = outcome
        .edits
        .iter()
        .map(|edit| (edit.start, edit.end))
        .collect();
    let result_spans: Vec<(usize, usize)> = outcome
        .edits
        .iter()
        .map(|edit| (edit.result_start, edit.result_end))
        .collect();
    let shift = |offset: usize| -> usize {
        let mut delta: isize = 0;
        for edit in &outcome.edits {
            // An insertion at a node's start goes before the node.
            if edit.end <= offset {
                delta += edit.new.len() as isize - (edit.end - edit.start) as isize;
            }
        }
        (offset as isize + delta) as usize
    };
    let outside_source: BTreeSet<Shape> = shapes(&before, &source_spans)
        .into_iter()
        .map(|shape| Shape {
            start: shift(shape.start),
            end: shift(shape.start) + (shape.end - shape.start),
            ..shape
        })
        .collect();
    let outside_result: BTreeSet<Shape> = shapes(&after, &result_spans).into_iter().collect();
    if outside_source != outside_result {
        let missing: Vec<_> = outside_source.difference(&outside_result).take(3).collect();
        let extra: Vec<_> = outside_result.difference(&outside_source).take(3).collect();
        return Err(format!(
            "nodes outside the edits differ: lost {missing:?}, gained {extra:?}"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Shape {
    start: usize,
    end: usize,
    depth: usize,
    kind: &'static str,
    text: String,
}

fn parse(text: &str) -> Result<Document<'_>, String> {
    let profile = wikitext_profile(text);
    parse_profiled(ProfiledParseInput {
        source: &profile,
        wikitext: text,
    })
    .map_err(|error| error.to_string())
}

/// The nodes and tag regions that no edit touches: none lies strictly inside
/// them, and they lie inside none.
fn shapes(document: &Document<'_>, edits: &[(usize, usize)]) -> Vec<Shape> {
    let touched = |start: usize, end: usize| {
        edits.iter().any(|(edit_start, edit_end)| {
            if edit_start == edit_end {
                start < *edit_start && *edit_start < end
            } else {
                *edit_start < end && start < *edit_end
            }
        })
    };
    let mut out = Vec::new();
    for node in document.nodes() {
        let span = node.span();
        if touched(span.start(), span.end()) {
            continue;
        }
        out.push(Shape {
            start: span.start(),
            end: span.end(),
            depth: node.depth(),
            kind: match node.kind() {
                NodeKind::Template(_) => "template",
                NodeKind::Parameter(_) => "parameter",
                NodeKind::InternalLink(_) => "link",
                NodeKind::Protected(_) => "protected",
            },
            text: document.text(span).to_string(),
        });
    }
    for region in document.tag_regions() {
        if touched(region.span.start(), region.span.end()) {
            continue;
        }
        out.push(Shape {
            start: region.span.start(),
            end: region.span.end(),
            depth: 0,
            kind: region.tag.name(),
            text: document.text(region.span).to_string(),
        });
    }
    out
}
