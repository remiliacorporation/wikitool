//! The classes an article's subject probably belongs to, and for each the
//! properties and relation types that apply: a checklist of the facts an
//! article of that kind states and the infobox fields that carry them.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use super::typing::{ProjectOntology, SkippedCheck};

pub const ONTOLOGY_SCOUT_SCHEMA_VERSION: &str = "ontology_scout_v1";

/// Where the class evidence came from, in the order the scout prefers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    /// The stub or draft passed to the scout.
    Stub,
    /// The page that already exists under the topic's title.
    ExistingPage,
    /// The comparable pages the scout found.
    ComparablePages,
    /// No page stated a class.
    None,
}

#[derive(Debug, Clone, Serialize)]
pub struct OntologyScout {
    pub schema_version: &'static str,
    pub ontology_commit: Option<String>,
    pub evidence_source: EvidenceSource,
    pub probable_classes: Vec<ProbableClass>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<SkippedCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbableClass {
    pub id: String,
    pub label: Option<String>,
    /// How many evidence pages state the class.
    pub page_count: usize,
    pub evidence: Vec<ClassEvidence>,
    pub ancestors: Vec<TermRef>,
    pub properties: Vec<ApplicableProperty>,
    pub relations: Vec<ApplicableRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ClassEvidence {
    /// `class_template` or `infobox`.
    pub via: &'static str,
    /// The page that states it; absent for the stub.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// The infobox's display type, for `infobox` evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infobox_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TermRef {
    pub id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplicableProperty {
    pub id: String,
    pub label: Option<String>,
    pub value_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardinality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_units: Option<Vec<String>>,
    /// The infobox field that carries it, when the display module maps one.
    pub field: Option<String>,
    /// Whether it is one of the wiki's own terms in the local module.
    pub local: bool,
    /// The class in the lineage the property applies through.
    pub applies_through: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplicableRelation {
    pub local: bool,
    pub id: String,
    pub label: Option<String>,
    pub inverse_label: Option<String>,
    pub range_classes: Vec<String>,
    pub field: Option<String>,
    pub applies_through: String,
}

/// A page the scout read, by title, with its wikitext.
pub struct ScoutPage<'a> {
    pub title: &'a str,
    pub wikitext: &'a str,
}

impl ProjectOntology {
    /// Infer the probable classes from the stub, else the existing page,
    /// else the comparable pages, and list what applies to each.
    pub fn scout(
        &self,
        stub: Option<&str>,
        existing: Option<ScoutPage<'_>>,
        comparables: &[ScoutPage<'_>],
    ) -> OntologyScout {
        let mut skipped = Vec::new();
        for module in [&self.local_module, &self.display_module] {
            if !module.is_loaded() {
                skipped.push(SkippedCheck {
                    check: module.title,
                    reason: format!(
                        "{} is not usable at {} ({}); its terms and fields are left out",
                        module.title, module.relative_path, module.status
                    ),
                });
            }
        }
        let mut evidence_source = EvidenceSource::None;
        let mut found: BTreeMap<String, BTreeSet<ClassEvidence>> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        let candidates: Vec<(EvidenceSource, EvidencePages<'_>)> = vec![
            (
                EvidenceSource::Stub,
                stub.map(|text| (None, text)).into_iter().collect(),
            ),
            (
                EvidenceSource::ExistingPage,
                existing
                    .map(|page| (Some(page.title), page.wikitext))
                    .into_iter()
                    .collect(),
            ),
            (
                EvidenceSource::ComparablePages,
                comparables
                    .iter()
                    .map(|page| (Some(page.title), page.wikitext))
                    .collect(),
            ),
        ];
        for (source, pages) in candidates {
            for (title, text) in pages {
                for (class, evidence) in self.page_classes(title, text) {
                    if !found.contains_key(&class) {
                        order.push(class.clone());
                    }
                    found.entry(class).or_default().insert(evidence);
                }
            }
            if !found.is_empty() {
                evidence_source = source;
                break;
            }
        }
        // Most-evidenced classes first; ties keep the order they were found in,
        // which puts a page's primary class first.
        let mut ranked: Vec<(usize, String)> = order.into_iter().enumerate().collect();
        ranked.sort_by(|(left_index, left), (right_index, right)| {
            page_count(&found[right])
                .cmp(&page_count(&found[left]))
                .then(left_index.cmp(right_index))
        });
        let probable_classes = ranked
            .into_iter()
            .map(|(_, class)| {
                let evidence: Vec<ClassEvidence> = found[&class].iter().cloned().collect();
                self.describe_class(&class, evidence)
            })
            .collect();
        OntologyScout {
            schema_version: ONTOLOGY_SCOUT_SCHEMA_VERSION,
            ontology_commit: self.commit().map(str::to_string),
            evidence_source,
            probable_classes,
            skipped,
        }
    }

    /// The classes one page states, from its class template and infoboxes.
    fn page_classes(&self, title: Option<&str>, wikitext: &str) -> Vec<(String, ClassEvidence)> {
        let Ok(statements) = self.page_statements(wikitext) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for call in &statements.class_calls {
            for argument in &call.positional {
                if argument.value.is_empty() {
                    continue;
                }
                let class = self.vocabulary.current(&argument.value);
                if self.vocabulary.term("classes", &class).is_some() {
                    out.push((
                        class,
                        ClassEvidence {
                            via: "class_template",
                            page: title.map(str::to_string),
                            infobox_type: None,
                        },
                    ));
                }
            }
        }
        for (_, kind) in &statements.infoboxes {
            let Some(class) = self
                .vocabulary
                .display_type(kind)
                .and_then(|entry| entry.get("class"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let class = self.vocabulary.current(class);
            if self.vocabulary.term("classes", &class).is_some() {
                out.push((
                    class,
                    ClassEvidence {
                        via: "infobox",
                        page: title.map(str::to_string),
                        infobox_type: Some(kind.clone()),
                    },
                ));
            }
        }
        out
    }

    fn describe_class(&self, class: &str, evidence: Vec<ClassEvidence>) -> ProbableClass {
        let lineage = self.vocabulary.lineage(class);
        let fields = self.fields_by_property(class);
        let mut properties = Vec::new();
        let mut relations = Vec::new();
        for (id, term) in self.vocabulary.statement_terms() {
            let scope_key = if term.section == "relations" {
                "domainClasses"
            } else {
                "appliesTo"
            };
            let Some(scope) = term.ids(scope_key) else {
                continue;
            };
            let scope: BTreeSet<String> = scope
                .into_iter()
                .map(|id| self.vocabulary.current(id))
                .collect();
            let Some(through) = lineage.iter().find(|class| scope.contains(*class)) else {
                continue;
            };
            let field = fields.get(id).cloned();
            if term.section == "relations" {
                relations.push(ApplicableRelation {
                    local: term.local,
                    id: id.to_string(),
                    label: term.label().map(str::to_string),
                    inverse_label: term.str_field("inverseLabel").map(str::to_string),
                    range_classes: term
                        .ids("rangeClasses")
                        .unwrap_or_default()
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    field,
                    applies_through: through.clone(),
                });
            } else {
                properties.push(ApplicableProperty {
                    local: term.local,
                    id: id.to_string(),
                    label: term.label().map(str::to_string),
                    value_type: term.str_field("valueType").map(str::to_string),
                    cardinality: term.str_field("cardinality").map(str::to_string),
                    value_scheme: term.str_field("valueScheme").map(str::to_string),
                    allowed_units: term
                        .ids("allowedUnits")
                        .map(|units| units.into_iter().map(str::to_string).collect()),
                    field,
                    applies_through: through.clone(),
                });
            }
        }
        // Terms an infobox field carries first, then by id.
        properties.sort_by(|left, right| {
            left.field
                .is_none()
                .cmp(&right.field.is_none())
                .then(left.id.cmp(&right.id))
        });
        relations.sort_by(|left, right| {
            left.field
                .is_none()
                .cmp(&right.field.is_none())
                .then(left.id.cmp(&right.id))
        });
        ProbableClass {
            id: class.to_string(),
            label: self.vocabulary.label(class).map(str::to_string),
            page_count: page_count(&evidence.iter().cloned().collect()),
            evidence,
            ancestors: lineage
                .iter()
                .skip(1)
                .map(|id| TermRef {
                    id: id.clone(),
                    label: self.vocabulary.label(id).map(str::to_string),
                })
                .collect(),
            properties,
            relations,
        }
    }

    /// The display field that carries each property, preferring a field the
    /// class's own kinds of page list, then the first field by name.
    fn fields_by_property(&self, class: &str) -> BTreeMap<String, String> {
        let lineage = self.vocabulary.lineage(class);
        let mut preferred: BTreeSet<&str> = BTreeSet::new();
        for (_, kind) in self.vocabulary.display_types() {
            let kind_class = kind
                .get("class")
                .and_then(Value::as_str)
                .map(|id| self.vocabulary.current(id));
            if kind_class.is_some_and(|id| lineage.contains(&id)) {
                preferred.extend(self.vocabulary.type_field_names(kind));
            }
        }
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        let mut fields = self.vocabulary.display_fields();
        fields.sort_by_key(|(name, _)| (!preferred.contains(name), *name));
        for (name, field) in fields {
            if let Some(property) = field.get("property").and_then(Value::as_str) {
                out.entry(self.vocabulary.current(property))
                    .or_insert_with(|| name.to_string());
            }
        }
        out
    }
}

fn page_count(evidence: &BTreeSet<ClassEvidence>) -> usize {
    let pages: BTreeSet<Option<&str>> = evidence.iter().map(|item| item.page.as_deref()).collect();
    pages.len()
}

/// Pages offered as one kind of evidence: each page's title (none for the
/// stub) and its wikitext.
type EvidencePages<'a> = Vec<(Option<&'a str>, &'a str)>;
