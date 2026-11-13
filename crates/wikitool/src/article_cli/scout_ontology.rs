//! The ontology part of `article scout`: the subject's probable classes and,
//! for each, the properties and relation types that apply. Present only when
//! the site adapter has `[ontology]` and the project has the vendored export.

use std::fs;

use anyhow::Result;
use serde::Serialize;
use wikitool_core::authoring::model::{ArticleScoutResult, LocalExistenceState};
use wikitool_core::filesystem::title_to_relative_path;
use wikitool_core::ontology::scout::{EvidenceSource, OntologyScout, ScoutPage};
use wikitool_core::ontology::typing::{OntologyAvailability, load_project_ontology};
use wikitool_core::runtime::ResolvedPaths;
use wikitool_core::site::SiteAdapter;

/// Classes the brief view lists.
const BRIEF_CLASS_LIMIT: usize = 3;

pub(super) fn build_scout_ontology(
    paths: &ResolvedPaths,
    adapter: &SiteAdapter,
    scout: &ArticleScoutResult,
    stub: Option<&str>,
) -> Result<Option<OntologyScout>> {
    let OntologyAvailability::Ready(project) = load_project_ontology(paths, adapter)? else {
        return Ok(None);
    };
    let read = |title: &str| -> Option<String> {
        let relative = title_to_relative_path(paths, title, false).ok()?;
        fs::read_to_string(paths.project_root.join(relative)).ok()
    };
    let existing_text = (scout.local_state == LocalExistenceState::ExactPageExists)
        .then(|| read(&scout.topic))
        .flatten();
    let comparable_texts: Vec<(String, String)> = scout
        .local_integration
        .comparable_pages
        .iter()
        .filter_map(|title| Some((title.clone(), read(title)?)))
        .collect();
    let comparables: Vec<ScoutPage<'_>> = comparable_texts
        .iter()
        .map(|(title, wikitext)| ScoutPage { title, wikitext })
        .collect();
    Ok(Some(project.scout(
        stub,
        existing_text.as_deref().map(|wikitext| ScoutPage {
            title: &scout.topic,
            wikitext,
        }),
        &comparables,
    )))
}

/// The compact form for the brief view: counts and the top classes.
#[derive(Debug, Serialize)]
pub(super) struct ScoutOntologyCard<'a> {
    ontology_commit: Option<&'a str>,
    evidence_source: EvidenceSource,
    probable_class_count: usize,
    top_classes: Vec<ScoutClassCard<'a>>,
}

#[derive(Debug, Serialize)]
struct ScoutClassCard<'a> {
    id: &'a str,
    label: Option<&'a str>,
    page_count: usize,
    property_count: usize,
    relation_count: usize,
    /// Properties and relations an infobox field carries, by field name.
    fields: Vec<&'a str>,
}

pub(super) fn scout_ontology_card(scout: &OntologyScout) -> ScoutOntologyCard<'_> {
    ScoutOntologyCard {
        ontology_commit: scout.ontology_commit.as_deref(),
        evidence_source: scout.evidence_source,
        probable_class_count: scout.probable_classes.len(),
        top_classes: scout
            .probable_classes
            .iter()
            .take(BRIEF_CLASS_LIMIT)
            .map(|class| {
                let mut fields: Vec<&str> = class
                    .properties
                    .iter()
                    .filter_map(|property| property.field.as_deref())
                    .chain(
                        class
                            .relations
                            .iter()
                            .filter_map(|relation| relation.field.as_deref()),
                    )
                    .collect();
                fields.sort_unstable();
                fields.dedup();
                ScoutClassCard {
                    id: &class.id,
                    label: class.label.as_deref(),
                    page_count: class.page_count,
                    property_count: class.properties.len(),
                    relation_count: class.relations.len(),
                    fields,
                }
            })
            .collect(),
    }
}

/// A scout view with the ontology object appended, when there is one.
#[derive(Debug, Serialize)]
pub(super) struct WithOntology<'a, T: Serialize, O: Serialize> {
    #[serde(flatten)]
    pub(super) view: &'a T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) ontology: Option<O>,
}

/// One line for the text view.
pub(super) fn scout_ontology_line(scout: &OntologyScout) -> String {
    let classes: Vec<String> = scout
        .probable_classes
        .iter()
        .take(BRIEF_CLASS_LIMIT)
        .map(|class| match &class.label {
            Some(label) => format!("{} ({label})", class.id),
            None => class.id.clone(),
        })
        .collect();
    format!(
        "article_scout.ontology.probable_classes: {} [{}]",
        if classes.is_empty() {
            "<none>".to_string()
        } else {
            classes.join(", ")
        },
        serde_json::to_value(scout.evidence_source)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default()
    )
}
