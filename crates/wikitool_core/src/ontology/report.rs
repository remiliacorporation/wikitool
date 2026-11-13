//! Evidence about the ontology from the wiki's own pages, for the intake that
//! moves the wiki's terms into the shared ontology: how many pages use each
//! local term, which infobox fields state no property yet, which values fail
//! their declared value type, and which class ids no module defines.
//!
//! The report reads every article in the content mirror through the same
//! statement extraction and checks as `article lint`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use crate::filesystem::{ScanOptions, scan_files};
use crate::runtime::ResolvedPaths;
use crate::support::normalize_path;

use super::typing::{ProjectOntology, SkippedCheck, UNKNOWN_CLASS, VALUE_TYPE_MISMATCH};
use super::vocabulary::TERM_SECTIONS;

pub const ONTOLOGY_REPORT_SCHEMA_VERSION: &str = "ontology_report_v1";
/// Example pages listed per row.
const EXAMPLE_LIMIT: usize = 3;

#[derive(Debug, Clone, Serialize)]
pub struct OntologyReport {
    pub schema_version: &'static str,
    pub ontology_commit: Option<String>,
    /// Articles read from the content mirror.
    pub page_count: usize,
    /// Articles that state a class or fill a typed infobox.
    pub typed_page_count: usize,
    /// Every term the local module defines, with the pages that use it.
    pub local_terms: Vec<LocalTermUsage>,
    /// Display fields with no `property`, with the pages that fill them.
    pub fields_without_property: Vec<FieldUsage>,
    /// Values that do not parse as their property's value type, by property.
    pub value_failures: Vec<ValueFailures>,
    /// Class-template ids that neither module defines.
    pub unknown_classes: Vec<UnknownClass>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<SkippedCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalTermUsage {
    pub id: String,
    pub section: &'static str,
    pub label: Option<String>,
    /// Whether the local module has a mappings entry for it.
    pub mapped: bool,
    /// Whether the shared ontology also defines it, so the shared one is used.
    pub shadowed: bool,
    pub page_count: usize,
    pub example_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldUsage {
    pub field: String,
    pub label: Option<String>,
    pub page_count: usize,
    pub example_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueFailures {
    pub property: String,
    pub label: Option<String>,
    pub value_type: Option<String>,
    pub failure_count: usize,
    pub page_count: usize,
    pub examples: Vec<ValueExample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueExample {
    pub title: String,
    /// The field and value as written.
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnknownClass {
    pub id: String,
    pub page_count: usize,
    pub example_titles: Vec<String>,
}

/// An article and its wikitext.
pub struct ReportPage {
    pub title: String,
    pub wikitext: String,
}

/// Read every article (main namespace, not a redirect) in the content mirror.
pub fn read_content_pages(paths: &ResolvedPaths) -> Result<Vec<ReportPage>> {
    let files = scan_files(
        paths,
        &ScanOptions {
            include_content: true,
            include_templates: false,
            custom_content_folders: Vec::new(),
        },
    )?;
    let mut pages = Vec::new();
    for file in files {
        if file.is_redirect || file.namespace != "Main" {
            continue;
        }
        let path = paths.project_root.join(&file.relative_path);
        let wikitext = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", normalize_path(&path)))?;
        pages.push(ReportPage {
            title: file.title,
            wikitext,
        });
    }
    pages.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(pages)
}

#[derive(Default)]
struct Tally {
    pages: BTreeSet<String>,
    count: usize,
    examples: Vec<ValueExample>,
}

impl Tally {
    fn add(&mut self, title: &str, evidence: Option<String>) {
        self.count += 1;
        self.pages.insert(title.to_string());
        if let Some(evidence) = evidence
            && self.examples.len() < EXAMPLE_LIMIT
            && !self.examples.iter().any(|example| example.title == title)
        {
            self.examples.push(ValueExample {
                title: title.to_string(),
                evidence,
            });
        }
    }

    fn example_titles(&self) -> Vec<String> {
        self.pages.iter().take(EXAMPLE_LIMIT).cloned().collect()
    }
}

impl ProjectOntology {
    pub fn report(&self, pages: &[ReportPage]) -> OntologyReport {
        let mut terms: BTreeMap<String, Tally> = BTreeMap::new();
        let mut fields: BTreeMap<String, Tally> = BTreeMap::new();
        let mut failures: BTreeMap<String, Tally> = BTreeMap::new();
        let mut unknown: BTreeMap<String, Tally> = BTreeMap::new();
        let mut typed_page_count = 0;
        // Which checks cannot run does not depend on the page.
        let skipped: Vec<SkippedCheck> = self.check_article("").skipped;

        for page in pages {
            let title = page.title.as_str();
            let Ok(statements) = self.page_statements(&page.wikitext) else {
                continue;
            };
            if !statements.class_calls.is_empty() || !statements.infoboxes.is_empty() {
                typed_page_count += 1;
            }
            let mut used: BTreeSet<String> = BTreeSet::new();
            for call in &statements.class_calls {
                for argument in &call.positional {
                    if !argument.value.is_empty() {
                        used.insert(self.vocabulary.current(&argument.value));
                    }
                }
            }
            for (call, kind_key) in &statements.infoboxes {
                let Some(kind) = self.vocabulary.display_type(kind_key) else {
                    continue;
                };
                if let Some(class) = kind.get("class").and_then(Value::as_str) {
                    used.insert(self.vocabulary.current(class));
                }
                for (field_name, _) in self.filled_fields(call, kind) {
                    let field = self.vocabulary.display_field(field_name);
                    match field
                        .and_then(|field| field.get("property"))
                        .and_then(Value::as_str)
                    {
                        Some(property) => {
                            used.insert(self.vocabulary.current(property));
                        }
                        None => {
                            if field.is_some() {
                                fields
                                    .entry(field_name.to_string())
                                    .or_default()
                                    .add(title, None);
                            }
                        }
                    }
                }
            }
            for id in used {
                terms.entry(id).or_default().add(title, None);
            }
            let typing = self.check_article(&page.wikitext);
            for finding in typing.findings {
                if finding.code == VALUE_TYPE_MISMATCH {
                    failures
                        .entry(finding.term)
                        .or_default()
                        .add(title, Some(finding.evidence));
                } else if finding.code == UNKNOWN_CLASS && finding.via == "class_template" {
                    unknown.entry(finding.term).or_default().add(title, None);
                }
            }
        }

        let mut local_terms = Vec::new();
        let mut seen = BTreeSet::new();
        for section in TERM_SECTIONS {
            for id in self.vocabulary.local_ids(section) {
                if !seen.insert(id.to_string()) {
                    continue;
                }
                let tally = terms.get(id);
                local_terms.push(LocalTermUsage {
                    id: id.to_string(),
                    section,
                    label: self
                        .vocabulary
                        .local_entry(section, id)
                        .and_then(|entry| entry.get("label"))
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    mapped: self.vocabulary.local_has_mapping(id),
                    shadowed: !self.vocabulary.shared_sections_of(id).is_empty(),
                    page_count: tally.map_or(0, |tally| tally.pages.len()),
                    example_titles: tally.map(Tally::example_titles).unwrap_or_default(),
                });
            }
        }
        local_terms.sort_by(|left, right| {
            right
                .page_count
                .cmp(&left.page_count)
                .then(left.id.cmp(&right.id))
        });

        let mut fields_without_property: Vec<FieldUsage> = self
            .vocabulary
            .display_fields()
            .into_iter()
            .filter(|(_, field)| !field.contains_key("property"))
            .map(|(name, field)| {
                let tally = fields.get(name);
                FieldUsage {
                    field: name.to_string(),
                    label: field
                        .get("label")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    page_count: tally.map_or(0, |tally| tally.pages.len()),
                    example_titles: tally.map(Tally::example_titles).unwrap_or_default(),
                }
            })
            .collect();
        fields_without_property.sort_by(|left, right| {
            right
                .page_count
                .cmp(&left.page_count)
                .then(left.field.cmp(&right.field))
        });

        let mut value_failures: Vec<ValueFailures> = failures
            .into_iter()
            .map(|(property, tally)| {
                let term = self.vocabulary.term("properties", &property);
                ValueFailures {
                    label: term.and_then(|term| term.label()).map(str::to_string),
                    value_type: term
                        .and_then(|term| term.str_field("valueType"))
                        .map(str::to_string),
                    failure_count: tally.count,
                    page_count: tally.pages.len(),
                    examples: tally.examples,
                    property,
                }
            })
            .collect();
        value_failures.sort_by(|left, right| {
            right
                .failure_count
                .cmp(&left.failure_count)
                .then(left.property.cmp(&right.property))
        });

        let mut unknown_classes: Vec<UnknownClass> = unknown
            .into_iter()
            .map(|(id, tally)| UnknownClass {
                page_count: tally.pages.len(),
                example_titles: tally.example_titles(),
                id,
            })
            .collect();
        unknown_classes.sort_by(|left, right| {
            right
                .page_count
                .cmp(&left.page_count)
                .then(left.id.cmp(&right.id))
        });

        OntologyReport {
            schema_version: ONTOLOGY_REPORT_SCHEMA_VERSION,
            ontology_commit: self.commit().map(str::to_string),
            page_count: pages.len(),
            typed_page_count,
            local_terms,
            fields_without_property,
            value_failures,
            unknown_classes,
            skipped,
        }
    }
}
