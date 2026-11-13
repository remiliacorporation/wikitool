//! Typing checks: whether the classes, infobox fields and values an article
//! states match the vendored ontology and the wiki's own ontology modules,
//! and whether those modules are consistent with the ontology.
//!
//! Everything here is gated on the site adapter's `[ontology]` section and
//! the vendored export. A wiki without them gets no checks and no findings.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;

use anyhow::{Context, Result, bail};
use mediawiki_wikitext::{NodeKind, ProfiledParseInput, parse_profiled};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::content_store::parsing::canonical_template_title;
use crate::runtime::ResolvedPaths;
use crate::site::adapter::model::{OntologyRules, SiteAdapter};
use crate::support::normalize_path;

use super::lua_data::parse_lua_data;
use super::values::{Verdict, check_value, display_parts, is_valid_date_format, wikitext_profile};
use super::vocabulary::{Term, TypeField, Vocabulary};

/// The wiki's own terms.
pub const LOCAL_MODULE_TITLE: &str = "Module:Ontology/local";
/// The wiki's presentation of the ontology: field names and kinds of page.
pub const DISPLAY_MODULE_TITLE: &str = "Module:Ontology/display";

/// The checks, by the name reports use.
pub const CHECK_CLASS_STATEMENTS: &str = "class_statements";
pub const CHECK_INFOBOX_FIELDS: &str = "infobox_fields";
pub const CHECK_FIELD_VALUES: &str = "field_values";
pub const CHECK_TERM_DEFINITIONS: &str = "term_definitions";

/// Finding codes. Each is stable; the message names the id it is about.
pub const UNKNOWN_CLASS: &str = "ontology.unknown_class";
pub const RENAMED_TERM: &str = "ontology.renamed_term";
pub const UNKNOWN_FIELD: &str = "ontology.unknown_field";
pub const MALFORMED_WIKIDATA_ID: &str = "ontology.malformed_wikidata_id";
pub const UNKNOWN_TYPE: &str = "ontology.unknown_type";
pub const PROPERTY_NOT_APPLICABLE: &str = "ontology.property_not_applicable";
pub const VALUE_TYPE_MISMATCH: &str = "ontology.value_type_mismatch";
pub const UNMAPPED_LOCAL_TERM: &str = "ontology.unmapped_local_term";
pub const LOCAL_SHADOWS_SHARED: &str = "ontology.local_shadows_shared";
pub const UNRESOLVED_FIELD_PROPERTY: &str = "ontology.unresolved_field_property";

/// Validate the typing keys of a site adapter's `[ontology]` section.
pub fn validate_typing_rules(rules: &OntologyRules) -> Result<()> {
    for (key, title) in [
        ("class_template", rules.class_template.as_deref()),
        (
            "typed_infobox_template",
            rules.typed_infobox_template.as_deref(),
        ),
    ] {
        if let Some(title) = title
            && template_key(title).is_none()
        {
            bail!("site-adapter [ontology] {key} {title:?} is not a template title");
        }
    }
    for (title, kind) in &rules.infobox_types {
        if template_key(title).is_none() {
            bail!(
                "site-adapter [ontology] infobox_types names {title:?}, which is not a template title"
            );
        }
        if kind.trim().is_empty() {
            bail!("site-adapter [ontology] infobox_types maps {title:?} to an empty type");
        }
    }
    for format in &rules.date_formats {
        if !is_valid_date_format(format) {
            bail!(
                "site-adapter [ontology] date_formats has {format:?}, which is not a strftime date format"
            );
        }
    }
    Ok(())
}

/// A template title as MediaWiki compares it: `Template:` prefix, spaces for
/// underscores, first letter upper case.
pub(crate) fn template_key(raw: &str) -> Option<String> {
    let title = canonical_template_title(raw)?;
    let body = title.strip_prefix("Template:")?;
    let mut chars = body.chars();
    let first = chars.next()?;
    Some(format!(
        "Template:{}{}",
        first.to_uppercase(),
        chars.as_str()
    ))
}

/// Whether the ontology checks can run in a project.
#[derive(Debug)]
pub enum OntologyAvailability {
    /// The site adapter has no `[ontology]` section.
    NotConfigured,
    /// `[ontology]` is set, but the vendored export is missing or unreadable.
    Unavailable {
        reason: String,
    },
    Ready(Box<ProjectOntology>),
}

/// A module read from the templates mirror.
#[derive(Debug, Clone, Serialize)]
pub struct ModuleStatus {
    pub title: &'static str,
    pub relative_path: String,
    /// `loaded`, `absent` or `invalid`.
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ModuleStatus {
    pub fn is_loaded(&self) -> bool {
        self.status == "loaded"
    }

    fn unavailable_reason(&self) -> Option<String> {
        match self.status {
            "loaded" => None,
            "absent" => Some(format!(
                "{} is not in the templates mirror at {}",
                self.title, self.relative_path
            )),
            _ => Some(format!(
                "{} at {} cannot be read: {}",
                self.title,
                self.relative_path,
                self.error.as_deref().unwrap_or("unknown error")
            )),
        }
    }
}

/// A check that did not run, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkippedCheck {
    pub check: &'static str,
    pub reason: String,
}

#[derive(Debug)]
pub struct ProjectOntology {
    pub(crate) rules: OntologyRules,
    pub(crate) vocabulary: Vocabulary,
    pub local_module: ModuleStatus,
    pub display_module: ModuleStatus,
    class_template: Option<String>,
    typed_infobox_template: Option<String>,
    infobox_types: BTreeMap<String, String>,
}

impl ProjectOntology {
    pub fn commit(&self) -> Option<&str> {
        self.vocabulary.commit()
    }

    /// Why the modules cannot be used, if they cannot.
    fn modules_unavailable(&self) -> Option<String> {
        let reasons: Vec<String> = [&self.local_module, &self.display_module]
            .into_iter()
            .filter_map(ModuleStatus::unavailable_reason)
            .collect();
        (!reasons.is_empty()).then(|| reasons.join("; "))
    }
}

/// Load the vendored export and the wiki's ontology modules.
pub fn load_project_ontology(
    paths: &ResolvedPaths,
    adapter: &SiteAdapter,
) -> Result<OntologyAvailability> {
    let Some(rules) = adapter.ontology.clone() else {
        return Ok(OntologyAvailability::NotConfigured);
    };
    let export_path = paths.project_root.join(super::EXPORT_PATH);
    let shared: Value = match fs::read_to_string(&export_path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(error) => {
                return Ok(OntologyAvailability::Unavailable {
                    reason: format!(
                        "the vendored ontology at {} does not parse: {error}",
                        super::EXPORT_PATH
                    ),
                });
            }
        },
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(OntologyAvailability::Unavailable {
                reason: format!(
                    "this project has no vendored ontology at {}; `wikitool ontology export` produces it",
                    super::EXPORT_PATH
                ),
            });
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read {}", normalize_path(&export_path)));
        }
    };
    let (local_module, local) = load_module(paths, LOCAL_MODULE_TITLE)?;
    let (display_module, display) = load_module(paths, DISPLAY_MODULE_TITLE)?;
    let infobox_types = rules
        .infobox_types
        .iter()
        .filter_map(|(title, kind)| Some((template_key(title)?, kind.trim().to_lowercase())))
        .collect();
    Ok(OntologyAvailability::Ready(Box::new(ProjectOntology {
        class_template: rules.class_template.as_deref().and_then(template_key),
        typed_infobox_template: rules
            .typed_infobox_template
            .as_deref()
            .and_then(template_key),
        infobox_types,
        rules,
        vocabulary: Vocabulary::new(shared, local, display),
        local_module,
        display_module,
    })))
}

fn load_module(paths: &ResolvedPaths, title: &'static str) -> Result<(ModuleStatus, Value)> {
    let relative_path = super::module_title_relative_path(paths, title)?;
    let path = paths.project_root.join(&relative_path);
    let empty = Value::Object(Map::new());
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok((
                ModuleStatus {
                    title,
                    relative_path,
                    status: "absent",
                    error: None,
                },
                empty,
            ));
        }
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", normalize_path(&path)));
        }
    };
    Ok(match parse_lua_data(&text, title) {
        Ok(value) => (
            ModuleStatus {
                title,
                relative_path,
                status: "loaded",
                error: None,
            },
            value,
        ),
        Err(error) => (
            ModuleStatus {
                title,
                relative_path,
                status: "invalid",
                error: Some(error.to_string()),
            },
            empty,
        ),
    })
}

/// One finding. Article findings carry the byte range of the template call
/// or argument they are about; module findings carry the term and module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypingFinding {
    pub code: &'static str,
    pub message: String,
    /// The id the finding is about.
    pub term: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<&'static str>,
    /// Which statement the finding is about: `class_template`, `infobox`
    /// or `module`.
    #[serde(skip)]
    pub via: &'static str,
    #[serde(skip)]
    pub start: usize,
    #[serde(skip)]
    pub end: usize,
    #[serde(skip)]
    pub evidence: String,
    #[serde(skip)]
    pub remediation: String,
}

/// The typing findings for one article.
#[derive(Debug, Clone, Default)]
pub struct ArticleTyping {
    pub findings: Vec<TypingFinding>,
    pub checks_run: Vec<&'static str>,
    pub skipped: Vec<SkippedCheck>,
}

/// A template call in an article: its title and arguments.
#[derive(Debug, Clone)]
pub(crate) struct TemplateCall {
    pub(crate) title: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) positional: Vec<Argument>,
    /// Named arguments; a repeated name keeps its last value, as MediaWiki does.
    pub(crate) named: BTreeMap<String, Argument>,
}

#[derive(Debug, Clone)]
pub(crate) struct Argument {
    pub(crate) value: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// The ontology statements an article makes through templates.
#[derive(Debug, Clone, Default)]
pub(crate) struct PageStatements {
    pub(crate) class_calls: Vec<TemplateCall>,
    /// Infobox calls with the display type key each one renders.
    pub(crate) infoboxes: Vec<(TemplateCall, String)>,
}

impl ProjectOntology {
    /// Find the class template and typed infobox calls at the top level of an
    /// article.
    pub(crate) fn page_statements(&self, wikitext: &str) -> Result<PageStatements, String> {
        let profile = wikitext_profile(wikitext);
        let document = parse_profiled(ProfiledParseInput {
            source: &profile,
            wikitext,
        })
        .map_err(|error| error.to_string())?;
        let mut statements = PageStatements::default();
        for node in document.nodes() {
            if node.parent().is_some() {
                continue;
            }
            let NodeKind::Template(template) = node.kind() else {
                continue;
            };
            let Some(title) = template_key(document.text(template.name)) else {
                continue;
            };
            let is_class = self.class_template.as_deref() == Some(title.as_str());
            let is_typed = self.typed_infobox_template.as_deref() == Some(title.as_str());
            let mapped = self.infobox_types.get(&title).cloned();
            if !is_class && !is_typed && mapped.is_none() {
                continue;
            }
            let mut call = TemplateCall {
                title,
                start: node.span().start(),
                end: node.span().end(),
                positional: Vec::new(),
                named: BTreeMap::new(),
            };
            // MediaWiki numbers unnamed arguments 1, 2, ... in order; `n=`
            // sets position n; a later argument for a position wins.
            let mut numbered: BTreeMap<usize, Argument> = BTreeMap::new();
            let mut next_position = 1usize;
            for argument in &template.arguments {
                let value = Argument {
                    value: document.text(argument.value).trim().to_string(),
                    start: argument.span.start(),
                    end: argument.span.end(),
                };
                match argument.name {
                    None => {
                        numbered.insert(next_position, value);
                        next_position += 1;
                    }
                    Some(name) => {
                        let name = document.text(name).trim().to_string();
                        if let Ok(index) = name.parse::<usize>()
                            && index >= 1
                            && name == index.to_string()
                        {
                            numbered.insert(index, value);
                            continue;
                        }
                        call.named.insert(name, value);
                    }
                }
            }
            // The positions a module reads as args[1], args[2], ... until the
            // first one that is missing.
            let mut position = 1;
            while let Some(value) = numbered.remove(&position) {
                call.positional.push(value);
                position += 1;
            }
            if is_class {
                // Ids are compared as the module sees them: comments and
                // markup gone, and the list ends at the first blank id.
                for argument in &mut call.positional {
                    argument.value = plain_text(&argument.value);
                }
                if let Some(blank) = call
                    .positional
                    .iter()
                    .position(|argument| argument.value.is_empty())
                {
                    call.positional.truncate(blank);
                }
                if let Some(wikidata) = call.named.get_mut("wikidata") {
                    wikidata.value = plain_text(&wikidata.value);
                }
            }
            if is_class {
                statements.class_calls.push(call);
            } else if let Some(kind) = mapped {
                statements.infoboxes.push((call, kind));
            } else if let Some(kind) = call.named.get("type").map(|type_arg| {
                display_parts(&type_arg.value)
                    .unwrap_or_default()
                    .join(" ")
                    .trim()
                    .to_lowercase()
            }) && !kind.is_empty()
            {
                statements.infoboxes.push((call, kind));
            }
        }
        Ok(statements)
    }

    /// Check one article.
    pub fn check_article(&self, wikitext: &str) -> ArticleTyping {
        match self.page_statements(wikitext) {
            Ok(statements) => self.check_statements(&statements),
            Err(error) => ArticleTyping {
                skipped: skip_all(&format!("the article's wikitext does not parse: {error}")),
                ..ArticleTyping::default()
            },
        }
    }

    /// Check the statements already read from one article.
    pub(crate) fn check_statements(&self, statements: &PageStatements) -> ArticleTyping {
        let mut out = ArticleTyping::default();
        if let Some(reason) = self.modules_unavailable() {
            out.skipped = skip_all(&reason);
            return out;
        }
        // Terms the article states, for the term-definition checks.
        let mut referenced: BTreeMap<String, (usize, usize, String)> = BTreeMap::new();

        if self.class_template.is_some() {
            out.checks_run.push(CHECK_CLASS_STATEMENTS);
            for call in &statements.class_calls {
                let before = out.findings.len();
                self.check_class_call(call, &mut out.findings, &mut referenced);
                mark_via(&mut out.findings[before..], "class_template");
            }
        } else {
            out.skipped.push(SkippedCheck {
                check: CHECK_CLASS_STATEMENTS,
                reason: "the site adapter's [ontology] names no class_template".to_string(),
            });
        }

        if self.typed_infobox_template.is_some() || !self.infobox_types.is_empty() {
            out.checks_run.push(CHECK_INFOBOX_FIELDS);
            out.checks_run.push(CHECK_FIELD_VALUES);
            for (call, kind) in &statements.infoboxes {
                let before = out.findings.len();
                self.check_infobox(call, kind, &mut out.findings, &mut referenced);
                mark_via(&mut out.findings[before..], "infobox");
            }
        } else {
            for check in [CHECK_INFOBOX_FIELDS, CHECK_FIELD_VALUES] {
                out.skipped.push(SkippedCheck {
                    check,
                    reason: "the site adapter's [ontology] names no typed_infobox_template or infobox_types".to_string(),
                });
            }
        }

        out.checks_run.push(CHECK_TERM_DEFINITIONS);
        for (id, (start, end, evidence)) in referenced {
            for mut finding in self.local_term_findings(&id) {
                finding.start = start;
                finding.end = end;
                finding.evidence.clone_from(&evidence);
                out.findings.push(finding);
            }
        }
        out
    }

    fn check_class_call(
        &self,
        call: &TemplateCall,
        findings: &mut Vec<TypingFinding>,
        referenced: &mut BTreeMap<String, (usize, usize, String)>,
    ) {
        let template = call.title.trim_start_matches("Template:");
        for argument in &call.positional {
            let id = argument.value.as_str();
            if id.is_empty() {
                continue;
            }
            let evidence = format!("{{{{{template}|{id}}}}}");
            let current = self.vocabulary.current(id);
            if current != id {
                findings.push(finding(
                    RENAMED_TERM,
                    format!(
                        "{{{{{template}}}}} names {id}, which the ontology renamed to {current}."
                    ),
                    id,
                    argument,
                    &evidence,
                    format!("Replace {id} with {current}."),
                ));
            }
            match self.vocabulary.find(&current) {
                Some(term) if term.section == "classes" => {
                    referenced
                        .entry(current.clone())
                        .or_insert((argument.start, argument.end, evidence));
                }
                Some(term) => findings.push(finding(
                    UNKNOWN_CLASS,
                    format!(
                        "{{{{{template}}}}} names {current}, which is among the {} of the ontology, not its classes.",
                        term.section
                    ),
                    &current,
                    argument,
                    &evidence,
                    "Name a class from Module:Ontology/data or Module:Ontology/local.".to_string(),
                )),
                None => findings.push(finding(
                    UNKNOWN_CLASS,
                    format!(
                        "{{{{{template}}}}} names {current}, which is not a class in Module:Ontology/data or {LOCAL_MODULE_TITLE}."
                    ),
                    &current,
                    argument,
                    &evidence,
                    format!("Use an existing class, or add {current} to {LOCAL_MODULE_TITLE} with a mappings entry first."),
                )),
            }
        }
        for (name, argument) in &call.named {
            let evidence = format!("{name}={}", argument.value);
            if name == "wikidata" {
                if !is_wikidata_item(&argument.value) {
                    findings.push(finding(
                        MALFORMED_WIKIDATA_ID,
                        format!(
                            "{{{{{template}}}}} has wikidata={}, which is not a Wikidata item id such as Q42.",
                            argument.value
                        ),
                        &argument.value,
                        argument,
                        &evidence,
                        "Give the item id alone: Q followed by digits.".to_string(),
                    ));
                }
                continue;
            }
            let known = self
                .vocabulary
                .display_field(name)
                .is_some_and(|field| field.contains_key("category"));
            if !known {
                findings.push(finding(
                    UNKNOWN_FIELD,
                    format!(
                        "{{{{{template}}}}} has the field {name}, which {DISPLAY_MODULE_TITLE} does not define as a relation field."
                    ),
                    name,
                    argument,
                    &evidence,
                    format!("Use a relation field from {DISPLAY_MODULE_TITLE}, or wikidata=."),
                ));
            }
        }
    }

    fn check_infobox(
        &self,
        call: &TemplateCall,
        kind_key: &str,
        findings: &mut Vec<TypingFinding>,
        referenced: &mut BTreeMap<String, (usize, usize, String)>,
    ) {
        let template = call.title.trim_start_matches("Template:");
        let call_span = Argument {
            value: String::new(),
            start: call.start,
            end: call.end,
        };
        let Some(kind) = self.vocabulary.display_type(kind_key) else {
            let (argument, evidence) = match call.named.get("type") {
                Some(argument) => (argument, format!("type={}", argument.value)),
                None => (&call_span, format!("{{{{{template}}}}}")),
            };
            findings.push(finding(
                UNKNOWN_TYPE,
                format!(
                    "{{{{{template}}}}} renders the type {kind_key:?}, which {DISPLAY_MODULE_TITLE} does not define."
                ),
                kind_key,
                argument,
                &evidence,
                format!("Use a type listed in {DISPLAY_MODULE_TITLE}."),
            ));
            return;
        };
        let Some(class) = kind.get("class").and_then(Value::as_str) else {
            return;
        };
        let class = self.vocabulary.current(class);
        let call_evidence = format!("{{{{{template}}}}}");
        if self.vocabulary.term("classes", &class).is_none() {
            findings.push(finding(
                UNKNOWN_CLASS,
                format!(
                    "The infobox type {kind_key:?} in {DISPLAY_MODULE_TITLE} is the class {class}, which is not a class in Module:Ontology/data or {LOCAL_MODULE_TITLE}."
                ),
                &class,
                &call_span,
                &call_evidence,
                format!("Add {class} to {LOCAL_MODULE_TITLE}, or point the type at an existing class."),
            ));
            return;
        }
        referenced
            .entry(class.clone())
            .or_insert((call.start, call.end, call_evidence));
        let lineage = self.vocabulary.lineage(&class);
        let class_label = self.vocabulary.label(&class).unwrap_or(&class).to_string();
        for (field, argument) in self.filled_fields(call, kind) {
            let field_name = field.name;
            let Some(property_id) = field.property else {
                continue;
            };
            let evidence = format!("{field_name}={}", argument.value);
            let current = self.vocabulary.current(property_id);
            let term = self
                .vocabulary
                .term("properties", &current)
                .or_else(|| self.vocabulary.term("relations", &current));
            let Some(term) = term else {
                findings.push(finding(
                    UNRESOLVED_FIELD_PROPERTY,
                    format!(
                        "The field {field_name} states {current}, which is neither in Module:Ontology/data nor {LOCAL_MODULE_TITLE}."
                    ),
                    &current,
                    argument,
                    &evidence,
                    format!("Add {current} to {LOCAL_MODULE_TITLE}, or map the field to an existing property in {DISPLAY_MODULE_TITLE}."),
                ));
                continue;
            };
            referenced.entry(current.clone()).or_insert((
                argument.start,
                argument.end,
                evidence.clone(),
            ));
            // The display module lists the field for this kind, so a property
            // that does not apply is a module defect; `ontology check` reports
            // it once, and lint only notes it here as a suggestion.
            if let Some(scope) = self.inapplicable_scope(&term, &lineage) {
                findings.push(finding(
                    PROPERTY_NOT_APPLICABLE,
                    format!(
                        "The field {field_name} states {current}, which does not apply to {class} ({class_label}) or its ancestors; it applies to {}. {DISPLAY_MODULE_TITLE} lists the field for {kind_key:?}; `wikitool ontology check` reports it.",
                        scope.join(", ")
                    ),
                    &current,
                    argument,
                    &evidence,
                    format!("Fix the property of {field_name} for {kind_key:?} in {DISPLAY_MODULE_TITLE}, or the term's appliesTo."),
                ));
            }
            if term.section == "properties" {
                self.check_field_value(field_name, &current, &term, argument, &evidence, findings);
            }
        }
    }

    fn check_field_value(
        &self,
        field_name: &str,
        property_id: &str,
        term: &Term<'_>,
        argument: &Argument,
        evidence: &str,
        findings: &mut Vec<TypingFinding>,
    ) {
        let Some(parts) = display_parts(&argument.value) else {
            return;
        };
        let many = term.str_field("cardinality") == Some("many");
        let values: Vec<String> = if many {
            parts
                .iter()
                .flat_map(|part| part.split(';'))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        } else if parts.is_empty() {
            Vec::new()
        } else {
            vec![parts.join(" ")]
        };
        let value_type = term.str_field("valueType").unwrap_or("unknown");
        for value in values {
            if let Verdict::Invalid(why) =
                check_value(&value, term, &self.vocabulary, &self.rules.date_formats)
            {
                findings.push(finding(
                    VALUE_TYPE_MISMATCH,
                    format!(
                        "The field {field_name} states {property_id} ({value_type}), but {value:?} {why}."
                    ),
                    property_id,
                    argument,
                    evidence,
                    format!("Write the value as a {value_type}."),
                ));
            }
        }
    }

    /// Mapping and shadowing findings for one local term.
    fn local_term_findings(&self, id: &str) -> Vec<TypingFinding> {
        let mut out = Vec::new();
        let local_sections: Vec<&'static str> = super::vocabulary::TERM_SECTIONS
            .iter()
            .copied()
            .filter(|section| self.vocabulary.local_entry(section, id).is_some())
            .collect();
        if local_sections.is_empty() {
            return out;
        }
        let shared = self.vocabulary.shared_sections_of(id);
        if !shared.is_empty() {
            out.push(module_finding(
                LOCAL_SHADOWS_SHARED,
                format!(
                    "{LOCAL_MODULE_TITLE} defines {id} among its {}, but the shared ontology already defines it among its {}; the shared term wins.",
                    local_sections.join(", "),
                    shared.join(", ")
                ),
                id,
                LOCAL_MODULE_TITLE,
                format!("Remove {id} from {LOCAL_MODULE_TITLE}."),
            ));
        } else if !self.vocabulary.local_has_mapping(id) {
            out.push(module_finding(
                UNMAPPED_LOCAL_TERM,
                format!("{id} in {LOCAL_MODULE_TITLE} has no mappings entry."),
                id,
                LOCAL_MODULE_TITLE,
                format!(
                    "Add a mappings entry for {id}: the nearest published term with skos:closeMatch, or sssom:NoTermFound."
                ),
            ));
        }
        out
    }

    /// Check the modules alone: every local term is mapped and shadows no
    /// shared term, and every display field and type resolves.
    pub fn check_modules(&self) -> ModuleCheck {
        let mut out = ModuleCheck::default();
        match self.local_module.unavailable_reason() {
            Some(reason) => {
                for check in ["local_mappings", "local_shadowing", "display_terms"] {
                    out.skipped.push(SkippedCheck {
                        check,
                        reason: reason.clone(),
                    });
                }
                return out;
            }
            None => {
                out.checks_run.push("local_mappings");
                out.checks_run.push("local_shadowing");
            }
        }
        let mut seen = BTreeSet::new();
        for section in super::vocabulary::TERM_SECTIONS {
            for id in self.vocabulary.local_ids(section) {
                if seen.insert(id.to_string()) {
                    out.findings.extend(self.local_term_findings(id));
                }
            }
        }
        if let Some(reason) = self.display_module.unavailable_reason() {
            out.skipped.push(SkippedCheck {
                check: "display_terms",
                reason,
            });
            return out;
        }
        out.checks_run.push("display_terms");
        for (name, field) in self.vocabulary.display_fields() {
            let Some(property) = field.get("property").and_then(Value::as_str) else {
                continue;
            };
            let current = self.vocabulary.current(property);
            if current != property {
                out.findings.push(module_finding(
                    RENAMED_TERM,
                    format!("The field {name} states {property}, which the ontology renamed to {current}."),
                    property,
                    DISPLAY_MODULE_TITLE,
                    format!("Replace {property} with {current} in {DISPLAY_MODULE_TITLE}."),
                ));
            }
            let resolves = self.vocabulary.term("properties", &current).is_some()
                || self.vocabulary.term("relations", &current).is_some();
            if !resolves {
                out.findings.push(module_finding(
                    UNRESOLVED_FIELD_PROPERTY,
                    format!(
                        "The field {name} states {current}, which is neither in Module:Ontology/data nor {LOCAL_MODULE_TITLE}."
                    ),
                    &current,
                    DISPLAY_MODULE_TITLE,
                    format!("Add {current} to {LOCAL_MODULE_TITLE}, or map the field to an existing property."),
                ));
            }
        }
        for (key, kind) in self.vocabulary.display_types() {
            let Some(class) = kind.get("class").and_then(Value::as_str) else {
                continue;
            };
            let current = self.vocabulary.current(class);
            if current != class {
                out.findings.push(module_finding(
                    RENAMED_TERM,
                    format!("The type {key:?} is the class {class}, which the ontology renamed to {current}."),
                    class,
                    DISPLAY_MODULE_TITLE,
                    format!("Replace {class} with {current} in {DISPLAY_MODULE_TITLE}."),
                ));
            }
            if self.vocabulary.term("classes", &current).is_none() {
                out.findings.push(module_finding(
                    UNKNOWN_CLASS,
                    format!(
                        "The type {key:?} is the class {current}, which is not a class in Module:Ontology/data or {LOCAL_MODULE_TITLE}."
                    ),
                    &current,
                    DISPLAY_MODULE_TITLE,
                    format!("Add {current} to {LOCAL_MODULE_TITLE}, or point the type at an existing class."),
                ));
                continue;
            }
            out.findings
                .extend(self.check_kind_fields(key, kind, &current));
        }
        out
    }
}

/// The module checks' result.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ModuleCheck {
    pub checks_run: Vec<&'static str>,
    pub skipped: Vec<SkippedCheck>,
    pub findings: Vec<TypingFinding>,
}

fn finding(
    code: &'static str,
    message: String,
    term: &str,
    argument: &Argument,
    evidence: &str,
    remediation: String,
) -> TypingFinding {
    TypingFinding {
        code,
        message,
        term: term.to_string(),
        module: None,
        via: "",
        start: argument.start,
        end: argument.end,
        evidence: evidence.to_string(),
        remediation,
    }
}

fn module_finding(
    code: &'static str,
    message: String,
    term: &str,
    module: &'static str,
    remediation: String,
) -> TypingFinding {
    TypingFinding {
        code,
        message,
        term: term.to_string(),
        module: Some(module),
        via: "module",
        start: 0,
        end: 0,
        evidence: String::new(),
        remediation,
    }
}

fn is_wikidata_item(value: &str) -> bool {
    value
        .strip_prefix('Q')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn mark_via(findings: &mut [TypingFinding], via: &'static str) {
    for finding in findings {
        finding.via = via;
    }
}

impl ProjectOntology {
    /// The fields of a kind of page that an infobox call fills: the names its
    /// sections list that the call gives a non-empty value.
    pub(crate) fn filled_fields<'c, 'k>(
        &'k self,
        call: &'c TemplateCall,
        kind: &'k Map<String, Value>,
    ) -> Vec<(TypeField<'k>, &'c Argument)> {
        self.vocabulary
            .type_fields(kind)
            .into_iter()
            .filter_map(|field| {
                // Module:Infobox skips a listed name with no field definition.
                if !field.known {
                    return None;
                }
                let argument = call.named.get(field.name)?;
                (!argument.value.is_empty()).then_some((field, argument))
            })
            .collect()
    }

    /// The classes a property or relation applies to, when none of a
    /// lineage's classes is among them; `None` when it applies or declares
    /// no scope (a qualifier, or a term without `appliesTo`).
    pub(crate) fn inapplicable_scope(
        &self,
        term: &Term<'_>,
        lineage: &[String],
    ) -> Option<Vec<String>> {
        let scope_key = if term.section == "relations" {
            "domainClasses"
        } else {
            "appliesTo"
        };
        let scope: BTreeSet<String> = term
            .ids(scope_key)?
            .into_iter()
            .map(|id| self.vocabulary.current(id))
            .collect();
        (!lineage.iter().any(|class| scope.contains(class))).then(|| scope.into_iter().collect())
    }

    /// The per-kind checks: each field a kind lists states a property that
    /// resolves and applies to the kind's class.
    fn check_kind_fields(
        &self,
        key: &str,
        kind: &Map<String, Value>,
        class: &str,
    ) -> Vec<TypingFinding> {
        let mut out = Vec::new();
        let lineage = self.vocabulary.lineage(class);
        for field in self.vocabulary.type_fields(kind) {
            let name = field.name;
            if !field.known {
                out.push(module_finding(
                    UNKNOWN_FIELD,
                    format!("The type {key:?} lists the field {name}, which {DISPLAY_MODULE_TITLE} does not define in its fields."),
                    name,
                    DISPLAY_MODULE_TITLE,
                    format!("Define {name} in the fields of {DISPLAY_MODULE_TITLE}, or remove it from {key:?}."),
                ));
            }
            let Some(property) = field.property else {
                continue;
            };
            let current = self.vocabulary.current(property);
            // A field's own property is checked with the fields; check an
            // override here.
            if field.overridden && current != property {
                out.push(module_finding(
                    RENAMED_TERM,
                    format!("The type {key:?} gives the field {name} the property {property}, which the ontology renamed to {current}."),
                    property,
                    DISPLAY_MODULE_TITLE,
                    format!("Replace {property} with {current} in {DISPLAY_MODULE_TITLE}."),
                ));
            }
            let term = self
                .vocabulary
                .term("properties", &current)
                .or_else(|| self.vocabulary.term("relations", &current));
            let Some(term) = term else {
                if field.overridden {
                    out.push(module_finding(
                        UNRESOLVED_FIELD_PROPERTY,
                        format!(
                            "The type {key:?} gives the field {name} the property {current}, which is neither in Module:Ontology/data nor {LOCAL_MODULE_TITLE}."
                        ),
                        &current,
                        DISPLAY_MODULE_TITLE,
                        format!("Add {current} to {LOCAL_MODULE_TITLE}, or give the field an existing property."),
                    ));
                }
                continue;
            };
            if let Some(scope) = self.inapplicable_scope(&term, &lineage) {
                out.push(module_finding(
                    PROPERTY_NOT_APPLICABLE,
                    format!(
                        "The type {key:?} lists the field {name}, whose property {current} does not apply to {class} or its ancestors; it applies to {}.",
                        scope.join(", ")
                    ),
                    &current,
                    DISPLAY_MODULE_TITLE,
                    format!("Give {name} a property that applies to {class} for {key:?}, or extend the term's appliesTo."),
                ));
            }
        }
        out
    }
}

fn skip_all(reason: &str) -> Vec<SkippedCheck> {
    [
        CHECK_CLASS_STATEMENTS,
        CHECK_INFOBOX_FIELDS,
        CHECK_FIELD_VALUES,
        CHECK_TERM_DEFINITIONS,
    ]
    .into_iter()
    .map(|check| SkippedCheck {
        check,
        reason: reason.to_string(),
    })
    .collect()
}

/// What a reader (and a module, after the preprocessor) sees of an argument:
/// comments and markup gone, whitespace collapsed.
fn plain_text(raw: &str) -> String {
    display_parts(raw)
        .map(|parts| parts.join(" "))
        .unwrap_or_else(|| raw.trim().to_string())
}
