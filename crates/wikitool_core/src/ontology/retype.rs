//! `article retype`: apply a typing plan to articles. Per page it states the
//! planned classes in the class template call and removes or adds
//! `[[Category:…]]` links, and changes nothing else. Every edit is a node
//! replacement, a node removed with its line, or an insertion between nodes,
//! resolved by the structural wikitext parser; every other byte is kept. A
//! page that cannot be edited that way is refused with the reason, never
//! changed in part.
//!
//! It reads a page's draft under `.wikitool/drafts/`, or else its mirror
//! file, and writes at most the draft. It never writes `wiki_content/`,
//! contacts no wiki and records no acceptance.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result, bail};
use mediawiki_wikitext::{
    Document, EditOrigin, LinkKind, Node, NodeId, NodeKind, ProfiledParseInput, ProtectedKind,
    RewritePlan, parse_profiled,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::TextDiff;

use crate::filesystem::{Namespace, namespace_from_title, title_to_relative_path};
use crate::runtime::ResolvedPaths;
use crate::site::load_site_adapter;
use crate::support::{atomic_write, compute_sha256, normalize_path};

use super::typing::{
    OntologyAvailability, ProjectOntology, RENAMED_TERM, SkippedCheck, UNKNOWN_CLASS,
    load_project_ontology, plain_text, template_key,
};
use super::values::wikitext_profile;

#[cfg(test)]
mod proof;
#[cfg(test)]
mod tests;

/// The plan file's schema.
pub const PLAN_SCHEMA: &str = "wikitool.article-retype-plan.v1";
/// The report's schema.
pub const REPORT_SCHEMA: &str = "wikitool.article-retype.v1";

/// Subclass hops followed to find a class's category, as `Module:Infobox` does.
const MAX_CATEGORY_HOPS: usize = 15;

/// A typing plan: what to state on each page.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetypePlan {
    pub schema: String,
    pub items: Vec<RetypePlanItem>,
}

/// What to state on one page.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetypePlanItem {
    pub title: String,
    /// The classes the class statement states, primary first. Empty keeps the
    /// classes of an existing call and adds no call.
    #[serde(default)]
    pub classes: Vec<String>,
    /// Relation fields of the class statement. A listed field is set to its
    /// values joined with `; `, or removed when the list is empty; a field
    /// not listed is kept as written.
    #[serde(default)]
    pub relations: BTreeMap<String, Vec<String>>,
    /// The subject's Wikidata item. Absent keeps the call's `wikidata=`; an
    /// empty string removes it.
    #[serde(default)]
    pub wikidata: Option<String>,
    /// Category links to remove. A page without one of them is refused.
    #[serde(default)]
    pub remove_categories: Vec<String>,
    /// Category links to add when the page lacks them.
    #[serde(default)]
    pub add_categories: Vec<String>,
    /// Also remove hand-written links to the categories that the class
    /// statement files the page in.
    #[serde(default)]
    pub drop_generated: bool,
}

/// Read and check a plan file's JSON.
pub fn parse_plan(text: &str) -> Result<RetypePlan> {
    let plan: RetypePlan = serde_json::from_str(text).context("the plan does not parse")?;
    if plan.schema != PLAN_SCHEMA {
        bail!(
            "the plan's schema is {:?}; expected {PLAN_SCHEMA:?}",
            plan.schema
        );
    }
    let mut seen = BTreeSet::new();
    for item in &plan.items {
        let title = normalize_title(&item.title);
        if title.is_empty() {
            bail!("a plan item has an empty title");
        }
        if !seen.insert(title.clone()) {
            bail!("the plan lists {title:?} more than once");
        }
    }
    Ok(plan)
}

/// A page title as MediaWiki compares it: spaces for underscores, runs of
/// whitespace collapsed, first letter upper case.
pub fn normalize_title(raw: &str) -> String {
    first_upper(
        &raw.replace('_', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn first_upper(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A category name as MediaWiki compares it, without the namespace.
fn normalize_category(raw: &str) -> String {
    let trimmed = raw.trim();
    let bare = match trimmed.split_once(':') {
        Some((namespace, rest)) if namespace.trim().eq_ignore_ascii_case("category") => rest,
        _ => trimmed,
    };
    normalize_title(bare)
}

/// One edit, with the bytes it replaces in the source and the bytes that
/// replace them in the result. Swapping each `new` back for its `old`, at
/// `result_start..result_end`, gives the source again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetypeEdit {
    /// `class_statement_replace`, `class_statement_insert`,
    /// `category_remove`, `category_drop_generated` or `category_add`.
    pub kind: &'static str,
    /// The categories a category edit is about; one line can hold several.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    pub start: usize,
    pub end: usize,
    pub result_start: usize,
    pub result_end: usize,
    pub old: String,
    pub new: String,
}

/// Why a page was not changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
    /// Where a link or call sits that cannot be edited: `template`,
    /// `parameter`, `link`, `includeonly`, `noinclude`, `onlyinclude`,
    /// `gallery`, `comment`, `nowiki`, `pre`, `source`, `syntaxhighlight`,
    /// or `multiple` for more than one class call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,
}

impl Refusal {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: None,
            category: None,
            term: None,
            start: None,
            end: None,
        }
    }
}

pub const REFUSED_REDIRECT: &str = "redirect";
pub const REFUSED_PARSE_FAILED: &str = "parse_failed";
pub const REFUSED_CATEGORY_NOT_EDITABLE: &str = "category_not_editable";
pub const REFUSED_CLASS_STATEMENT_NOT_EDITABLE: &str = "class_statement_not_editable";
pub const REFUSED_REMOVAL_MISSING: &str = "removal_missing";
pub const REFUSED_INVALID_PLAN: &str = "invalid_plan";
pub const REFUSED_PAGE_MISSING: &str = "page_missing";
pub const REFUSED_NOT_IN_PLAN: &str = "not_in_plan";
pub const REFUSED_NOT_AN_ARTICLE: &str = "not_an_article";
pub const REFUSED_REWRITE_FAILED: &str = "rewrite_failed";

/// A typing finding on the result, as `article lint` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetypeFinding {
    pub code: &'static str,
    pub message: String,
    pub term: String,
    /// `class_template`, `infobox` or `module`.
    pub via: &'static str,
    pub start: usize,
    pub end: usize,
    pub evidence: String,
    pub remediation: String,
}

/// The typing checks on the result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RetypeTyping {
    pub checks_run: Vec<&'static str>,
    pub skipped: Vec<SkippedCheck>,
    pub findings: Vec<RetypeFinding>,
}

/// One page's outcome: the result text, and the edits, refusals and notes
/// that explain it. A refused page's result is its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRetype {
    pub result: String,
    pub edits: Vec<RetypeEdit>,
    pub refusals: Vec<Refusal>,
    pub notes: Vec<String>,
    pub typing: RetypeTyping,
}

impl PageRetype {
    fn refused(source: &str, ontology: &ProjectOntology, refusals: Vec<Refusal>) -> Self {
        Self {
            result: source.to_string(),
            edits: Vec::new(),
            refusals,
            notes: Vec::new(),
            typing: typing_of(ontology, source),
        }
    }
}

fn typing_of(ontology: &ProjectOntology, text: &str) -> RetypeTyping {
    let typing = ontology.check_article(text);
    RetypeTyping {
        checks_run: typing.checks_run,
        skipped: typing.skipped,
        findings: typing
            .findings
            .into_iter()
            .map(|finding| RetypeFinding {
                code: finding.code,
                message: finding.message,
                term: finding.term,
                via: finding.via,
                start: finding.start,
                end: finding.end,
                evidence: finding.evidence,
                remediation: finding.remediation,
            })
            .collect(),
    }
}

/// A `[[Category:…]]` link on the page, editable or not.
struct CategoryLink {
    /// The link node; `None` inside a comment or literal region, which the
    /// parser does not read as wikitext.
    node: Option<NodeId>,
    name: String,
    start: usize,
    end: usize,
    sort_key: bool,
    /// Where it sits when it cannot be edited.
    context: Option<&'static str>,
}

/// Apply one plan item to one page's text.
pub fn retype_text(ontology: &ProjectOntology, item: &RetypePlanItem, source: &str) -> PageRetype {
    let invalid = validate_item(item);
    if !invalid.is_empty() {
        return PageRetype::refused(source, ontology, invalid);
    }
    let Some(class_key) = ontology.class_template().map(str::to_string) else {
        return PageRetype::refused(
            source,
            ontology,
            vec![Refusal::new(
                REFUSED_INVALID_PLAN,
                "the site adapter's [ontology] names no class_template",
            )],
        );
    };
    let profile = wikitext_profile(source);
    let document = match parse_profiled(ProfiledParseInput {
        source: &profile,
        wikitext: source,
    }) {
        Ok(document) => document,
        Err(error) => {
            return PageRetype::refused(
                source,
                ontology,
                vec![Refusal::new(
                    REFUSED_PARSE_FAILED,
                    format!("the page's wikitext does not parse: {error}"),
                )],
            );
        }
    };
    if document.redirect().is_some() {
        return PageRetype::refused(
            source,
            ontology,
            vec![Refusal::new(REFUSED_REDIRECT, "the page is a redirect")],
        );
    }
    let statements = match ontology.page_statements(source) {
        Ok(statements) => statements,
        Err(error) => {
            return PageRetype::refused(
                source,
                ontology,
                vec![Refusal::new(
                    REFUSED_PARSE_FAILED,
                    format!("the page's wikitext does not parse: {error}"),
                )],
            );
        }
    };

    let mut editor = Editor {
        document: &document,
        source,
        eol: if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        },
        plan: RewritePlan::new(),
        kinds: BTreeMap::new(),
        insert_kinds: Vec::new(),
        refusals: Vec::new(),
        notes: Vec::new(),
    };

    // The class statement.
    let class_calls: Vec<&Node> = document
        .nodes()
        .iter()
        .filter(|node| match node.kind() {
            NodeKind::Template(template) => {
                template_key(document.text(template.name)).as_deref() == Some(class_key.as_str())
            }
            _ => false,
        })
        .collect();
    let template_name = class_key.trim_start_matches("Template:").to_string();
    let mut final_classes: Vec<String> = Vec::new();
    let mut has_call = false;
    match class_calls.as_slice() {
        [] => {
            if !item.classes.is_empty() {
                if let Some(reason) = typed_infobox_states(ontology, &statements, item) {
                    editor.notes.push(reason);
                } else {
                    editor.insert_class_call(ontology, &template_name, item);
                    final_classes = item
                        .classes
                        .iter()
                        .map(|id| id.trim().to_string())
                        .collect();
                    has_call = true;
                }
            } else if named_plan(item).values().any(Option::is_some) {
                editor.refusals.push(Refusal::new(
                    REFUSED_INVALID_PLAN,
                    "the plan sets relations or wikidata but names no classes, and the page has no class statement to set them on",
                ));
            }
        }
        [call] => {
            if let Some(context) = node_context(&document, call) {
                let mut refusal = Refusal::new(
                    REFUSED_CLASS_STATEMENT_NOT_EDITABLE,
                    format!("the {{{{{template_name}}}}} call is inside a {context}"),
                );
                refusal.context = Some(context);
                refusal.start = Some(call.span().start());
                refusal.end = Some(call.span().end());
                editor.refusals.push(refusal);
            } else {
                has_call = true;
                final_classes = if item.classes.is_empty() {
                    statements
                        .class_calls
                        .first()
                        .map(|existing| {
                            existing
                                .positional
                                .iter()
                                .map(|argument| argument.value.clone())
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    item.classes
                        .iter()
                        .map(|id| id.trim().to_string())
                        .collect()
                };
                editor.replace_class_call(call, item);
            }
        }
        [first, ..] => {
            let mut refusal = Refusal::new(
                REFUSED_CLASS_STATEMENT_NOT_EDITABLE,
                format!(
                    "the page has {} {{{{{template_name}}}}} calls",
                    class_calls.len()
                ),
            );
            refusal.context = Some("multiple");
            refusal.start = Some(first.span().start());
            refusal.end = Some(first.span().end());
            editor.refusals.push(refusal);
        }
    }

    // Category links.
    let generated: Vec<String> = if has_call {
        generated_categories(ontology, &final_classes)
    } else {
        Vec::new()
    };
    let remove: Vec<String> = dedupe(
        item.remove_categories
            .iter()
            .map(|name| normalize_category(name)),
    );
    let add: Vec<String> = dedupe(
        item.add_categories
            .iter()
            .map(|name| normalize_category(name)),
    );
    if item.drop_generated
        && let Some(name) = add.iter().find(|name| generated.contains(name))
    {
        editor.refusals.push(Refusal::new(
            REFUSED_INVALID_PLAN,
            format!(
                "the plan adds the category {name:?}, which drop_generated removes because the class statement files the page in it"
            ),
        ));
    }
    let links = category_links(&document);
    let mut relevant: BTreeSet<&str> = remove
        .iter()
        .chain(add.iter())
        .map(String::as_str)
        .collect();
    if item.drop_generated {
        relevant.extend(generated.iter().map(String::as_str));
    }
    for link in &links {
        if let Some(context) = link.context
            && relevant.contains(link.name.as_str())
        {
            let mut refusal = Refusal::new(
                REFUSED_CATEGORY_NOT_EDITABLE,
                format!(
                    "a link to the category {:?} is inside a {context}",
                    link.name
                ),
            );
            refusal.context = Some(context);
            refusal.category = Some(link.name.clone());
            refusal.start = Some(link.start);
            refusal.end = Some(link.end);
            editor.refusals.push(refusal);
        }
    }
    let editable: Vec<&CategoryLink> = links
        .iter()
        .filter(|link| link.context.is_none() && link.node.is_some())
        .collect();
    let mut removed: BTreeSet<NodeId> = BTreeSet::new();
    for name in &remove {
        let matching: Vec<&&CategoryLink> =
            editable.iter().filter(|link| &link.name == name).collect();
        if matching.is_empty() {
            if !links.iter().any(|link| &link.name == name) {
                let mut refusal = Refusal::new(
                    REFUSED_REMOVAL_MISSING,
                    format!(
                        "the plan removes the category {name:?}, which the page has no link to"
                    ),
                );
                refusal.category = Some(name.clone());
                editor.refusals.push(refusal);
            }
            continue;
        }
        for link in matching {
            let node = link.node.expect("editable links have nodes");
            if removed.insert(node) {
                editor.remove_link(node, "category_remove", name);
            }
        }
    }
    if item.drop_generated {
        for link in &editable {
            let node = link.node.expect("editable links have nodes");
            if !generated.contains(&link.name) || removed.contains(&node) {
                continue;
            }
            if link.sort_key {
                editor.notes.push(format!(
                    "kept the link to the category {:?}: its sort key is not in the generated link",
                    link.name
                ));
                continue;
            }
            removed.insert(node);
            editor.remove_link(node, "category_drop_generated", &link.name);
        }
    }
    let present: BTreeSet<&str> = editable
        .iter()
        .filter(|link| !removed.contains(&link.node.expect("editable links have nodes")))
        .map(|link| link.name.as_str())
        .collect();
    let to_add: Vec<&String> = add
        .iter()
        .filter(|name| {
            let here = present.contains(name.as_str());
            if here {
                editor
                    .notes
                    .push(format!("the page already links the category {name:?}"));
            }
            !here
        })
        .collect();
    if !editor.refusals.is_empty() {
        return PageRetype::refused(source, ontology, editor.refusals);
    }
    if !to_add.is_empty() {
        let anchor = editable
            .iter()
            .rev()
            .find(|link| !removed.contains(&link.node.expect("editable links have nodes")));
        if let Err(error) = editor.add_links(anchor.map(|link| link.end), &to_add) {
            return PageRetype::refused(source, ontology, vec![rewrite_refusal(&error)]);
        }
    }

    let notes = std::mem::take(&mut editor.notes);
    let (result, edits) = match editor.finish() {
        Ok(done) => done,
        Err(error) => return PageRetype::refused(source, ontology, vec![rewrite_refusal(&error)]),
    };

    // The typing checks on the result decide whether the planned classes
    // may be stated.
    let typing = typing_of(ontology, &result);
    let planned: BTreeSet<String> = item
        .classes
        .iter()
        .flat_map(|id| {
            let id = id.trim().to_string();
            let current = ontology.vocabulary.current(&id);
            [id, current]
        })
        .collect();
    let refusals: Vec<Refusal> = typing
        .findings
        .iter()
        .filter(|finding| {
            finding.via == "class_template"
                && (finding.code == UNKNOWN_CLASS || finding.code == RENAMED_TERM)
                && planned.contains(&finding.term)
        })
        .map(|finding| {
            let mut refusal = Refusal::new(finding.code, finding.message.clone());
            refusal.term = Some(finding.term.clone());
            refusal
        })
        .collect();
    if !refusals.is_empty() {
        return PageRetype::refused(source, ontology, refusals);
    }
    PageRetype {
        result,
        edits,
        refusals: Vec::new(),
        notes,
        typing,
    }
}

fn dedupe(names: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    names.filter(|name| seen.insert(name.clone())).collect()
}

const ID_FORBIDDEN: &[char] = &['|', '{', '}', '[', ']', '<', '>', '=', '\n', '\r'];
const VALUE_FORBIDDEN: &[char] = &['|', '{', '}', ';', '\n', '\r'];
const CATEGORY_FORBIDDEN: &[char] = &['[', ']', '{', '}', '|', '#', '<', '>', '\n', '\r'];

fn validate_item(item: &RetypePlanItem) -> Vec<Refusal> {
    let mut out = Vec::new();
    let mut invalid = |message: String| out.push(Refusal::new(REFUSED_INVALID_PLAN, message));
    let mut classes = BTreeSet::new();
    for id in &item.classes {
        let id = id.trim();
        if id.is_empty() || id.contains(ID_FORBIDDEN) {
            invalid(format!(
                "the class id {id:?} is empty or holds wikitext markup"
            ));
        } else if !classes.insert(id) {
            invalid(format!("the plan names the class {id} twice"));
        }
    }
    for (name, values) in &item.relations {
        let field = name.trim();
        if field.is_empty()
            || field == "wikidata"
            || field.bytes().all(|byte| byte.is_ascii_digit())
            || !field
                .chars()
                .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | ' '))
        {
            invalid(format!("{name:?} is not a relation field name"));
        }
        for value in values {
            if value.trim().is_empty() || value.contains(VALUE_FORBIDDEN) {
                invalid(format!(
                    "the {name} value {value:?} is empty or holds a separator or markup"
                ));
            }
        }
    }
    if let Some(wikidata) = &item.wikidata
        && wikidata.contains(VALUE_FORBIDDEN)
    {
        invalid(format!("wikidata {wikidata:?} holds a separator or markup"));
    }
    for name in item.remove_categories.iter().chain(&item.add_categories) {
        if normalize_category(name).is_empty()
            || name.contains(CATEGORY_FORBIDDEN)
            || name.trim().starts_with(':')
        {
            invalid(format!("{name:?} is not a category name"));
        }
    }
    let remove: BTreeSet<String> = item
        .remove_categories
        .iter()
        .map(|name| normalize_category(name))
        .collect();
    for name in &item.add_categories {
        let name = normalize_category(name);
        if remove.contains(&name) {
            invalid(format!(
                "the plan both removes and adds the category {name:?}"
            ));
        }
    }
    out
}

/// The named arguments the plan sets: a value, or `None` to remove it.
fn named_plan(item: &RetypePlanItem) -> BTreeMap<String, Option<String>> {
    let mut out: BTreeMap<String, Option<String>> = item
        .relations
        .iter()
        .map(|(name, values)| {
            let joined = values
                .iter()
                .map(|value| value.trim())
                .collect::<Vec<_>>()
                .join("; ");
            (
                name.trim().to_string(),
                (!joined.is_empty()).then_some(joined),
            )
        })
        .collect();
    if let Some(wikidata) = &item.wikidata {
        let wikidata = wikidata.trim();
        out.insert(
            "wikidata".to_string(),
            (!wikidata.is_empty()).then(|| wikidata.to_string()),
        );
    }
    out
}

/// When the page should get no class call: its typed infobox already states
/// the plan's only class, and the plan sets nothing else on the call.
fn typed_infobox_states(
    ontology: &ProjectOntology,
    statements: &super::typing::PageStatements,
    item: &RetypePlanItem,
) -> Option<String> {
    let [only] = item.classes.as_slice() else {
        return None;
    };
    if named_plan(item).values().any(Option::is_some) {
        return None;
    }
    let only = ontology.vocabulary.current(only.trim());
    statements.infoboxes.iter().find_map(|(call, kind)| {
        let class = ontology
            .vocabulary
            .display_type(kind)?
            .get("class")?
            .as_str()?;
        (ontology.vocabulary.current(class) == only).then(|| {
            format!(
                "added no class statement: {} already states {only}",
                call.title.trim_start_matches("Template:")
            )
        })
    })
}

/// The categories a class statement files a page in, as `Module:Infobox`'s
/// `instanceOf` computes them: each class's entry in the display module's
/// `classes`, or its nearest listed ancestor's, and for a `small` category
/// also the nearest ancestor category that is not small.
fn generated_categories(ontology: &ProjectOntology, classes: &[String]) -> Vec<String> {
    let vocabulary = &ontology.vocabulary;
    let mut out = Vec::new();
    let mut add = |class: Option<String>| {
        let Some(class) = class else {
            return;
        };
        if let Some(name) = vocabulary
            .display_class(&class)
            .and_then(|entry| entry.get("category"))
            .and_then(Value::as_str)
        {
            let name = normalize_category(name);
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
    };
    for id in classes {
        if vocabulary.term("classes", id).is_none() {
            continue;
        }
        let owner = category_class(ontology, id, false);
        let small = owner
            .as_deref()
            .and_then(|owner| vocabulary.display_class(owner))
            .is_some_and(is_small);
        if small {
            let parent = owner
                .as_deref()
                .and_then(|owner| superclass(ontology, owner));
            add(owner);
            add(parent.and_then(|parent| category_class(ontology, &parent, true)));
        } else {
            add(owner);
        }
    }
    out
}

fn is_small(entry: &serde_json::Map<String, Value>) -> bool {
    entry.get("small").and_then(Value::as_bool) == Some(true)
}

fn superclass(ontology: &ProjectOntology, class: &str) -> Option<String> {
    ontology
        .vocabulary
        .find(class)?
        .ids("subClassOf")?
        .first()
        .map(|id| id.to_string())
}

fn category_class(ontology: &ProjectOntology, id: &str, skip_small: bool) -> Option<String> {
    let mut current = id.to_string();
    for _ in 0..MAX_CATEGORY_HOPS {
        if let Some(entry) = ontology.vocabulary.display_class(&current)
            && !(skip_small && is_small(entry))
        {
            return Some(current);
        }
        current = superclass(ontology, &current)?;
    }
    None
}

/// Where a node sits when the command may not edit it: inside another node,
/// or inside a transclusion-control or gallery region.
fn node_context(document: &Document<'_>, node: &Node) -> Option<&'static str> {
    if let Some(parent) = node.parent().and_then(|parent| document.node(parent)) {
        return Some(match parent.kind() {
            NodeKind::Template(_) => "template",
            NodeKind::Parameter(_) => "parameter",
            NodeKind::InternalLink(_) => "link",
            NodeKind::Protected(region) => protected_name(region.kind),
        });
    }
    let span = node.span();
    document
        .tag_regions()
        .iter()
        .find(|region| region.span.start() < span.end() && span.start() < region.span.end())
        .map(|region| region.tag.name())
}

fn protected_name(kind: ProtectedKind) -> &'static str {
    match kind {
        ProtectedKind::Comment => "comment",
        ProtectedKind::Nowiki => "nowiki",
        ProtectedKind::Pre => "pre",
        ProtectedKind::Source => "source",
        ProtectedKind::SyntaxHighlight => "syntaxhighlight",
    }
}

/// The category a category link files the page in, or `None` for a link to
/// the category page (`[[:Category:…]]`) or another kind of link.
fn link_category(document: &Document<'_>, node: &Node) -> Option<(String, bool)> {
    let NodeKind::InternalLink(link) = node.kind() else {
        return None;
    };
    if link.kind != LinkKind::Category || link.leading_colon {
        return None;
    }
    let title = document.text(link.title);
    let (_, name) = title.split_once(':')?;
    Some((normalize_title(name), !link.components.is_empty()))
}

/// Every category link on the page, with the links that comments and
/// literal regions hold, which the parser leaves unread.
fn category_links(document: &Document<'_>) -> Vec<CategoryLink> {
    let mut out = Vec::new();
    for node in document.nodes() {
        if let Some((name, sort_key)) = link_category(document, node) {
            out.push(CategoryLink {
                node: Some(node.id()),
                name,
                start: node.span().start(),
                end: node.span().end(),
                sort_key,
                context: node_context(document, node),
            });
        }
        if let NodeKind::Protected(region) = node.kind() {
            protected_links(
                document.text(region.inner),
                region.inner.start(),
                protected_name(region.kind),
                &mut out,
            );
        }
    }
    out
}

fn protected_links(text: &str, offset: usize, context: &'static str, out: &mut Vec<CategoryLink>) {
    let profile = wikitext_profile(text);
    let Ok(document) = parse_profiled(ProfiledParseInput {
        source: &profile,
        wikitext: text,
    }) else {
        return;
    };
    for node in document.nodes() {
        if let Some((name, sort_key)) = link_category(&document, node) {
            out.push(CategoryLink {
                node: None,
                name,
                start: offset + node.span().start(),
                end: offset + node.span().end(),
                sort_key,
                context: Some(context),
            });
        }
        if let NodeKind::Protected(region) = node.kind() {
            protected_links(
                document.text(region.inner),
                offset + region.inner.start(),
                context,
                out,
            );
        }
    }
}

/// The plan for one page as it is built, and what each edit is for.
struct Editor<'d, 's> {
    document: &'d Document<'s>,
    source: &'s str,
    eol: &'static str,
    plan: RewritePlan,
    kinds: BTreeMap<NodeId, (&'static str, String)>,
    insert_kinds: Vec<(&'static str, Option<String>)>,
    refusals: Vec<Refusal>,
    notes: Vec<String>,
}

impl Editor<'_, '_> {
    fn insert(
        &mut self,
        offset: usize,
        text: String,
        kind: &'static str,
        category: Option<String>,
    ) {
        self.plan.insert(offset, text);
        self.insert_kinds.push((kind, category));
    }

    fn remove_link(&mut self, node: NodeId, kind: &'static str, category: &str) {
        if self.plan.remove_line(node).is_ok() {
            self.kinds.insert(node, (kind, category.to_string()));
        }
    }

    /// Whether a byte offset falls strictly inside a node.
    fn inside_node(&self, offset: usize) -> bool {
        self.document
            .nodes()
            .iter()
            .any(|node| node.span().start() < offset && offset < node.span().end())
    }

    /// The offset of the line break that ends the line holding `offset`, or
    /// the end of the source.
    fn line_end(&self, offset: usize) -> usize {
        let bytes = self.source.as_bytes();
        match bytes[offset..].iter().position(|byte| *byte == b'\n') {
            Some(index) => {
                let newline = offset + index;
                if newline > offset && bytes[newline - 1] == b'\r' {
                    newline - 1
                } else {
                    newline
                }
            }
            None => bytes.len(),
        }
    }

    fn line_start(&self, offset: usize) -> usize {
        self.source.as_bytes()[..offset]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1)
    }

    /// Where a new class call goes: on its own line after the short
    /// description, else before the first infobox, else at the top.
    fn insert_class_call(
        &mut self,
        ontology: &ProjectOntology,
        template_name: &str,
        item: &RetypePlanItem,
    ) {
        let mut text = format!("{{{{{template_name}");
        for class in &item.classes {
            text.push('|');
            text.push_str(class.trim());
        }
        for (name, value) in named_plan(item) {
            if let Some(value) = value {
                text.push_str(&format!("|{name}={value}"));
            }
        }
        text.push_str("}}");

        let top_level =
            |node: &&Node| node.parent().is_none() && node_context(self.document, node).is_none();
        let templates: Vec<(&Node, &str)> = self
            .document
            .nodes()
            .iter()
            .filter(top_level)
            .filter_map(|node| match node.kind() {
                NodeKind::Template(template) => Some((node, self.document.text(template.name))),
                _ => None,
            })
            .collect();
        let short_description = templates.iter().find(|(_, name)| {
            let name = name.trim();
            name.get(..10)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("SHORTDESC:"))
                || template_key(name).as_deref() == Some("Template:Short description")
        });
        if let Some((node, _)) = short_description {
            let mut offset = self.line_end(node.span().end());
            if self.inside_node(offset) {
                offset = node.span().end();
            }
            let eol = self.eol;
            self.insert(
                offset,
                format!("{eol}{text}"),
                "class_statement_insert",
                None,
            );
            return;
        }
        let infobox = templates.iter().find(|(_, name)| {
            template_key(name).is_some_and(|key| {
                ontology.is_typed_infobox(&key) || key.starts_with("Template:Infobox")
            })
        });
        let offset = match infobox {
            Some((node, _)) => {
                let start = self.line_start(node.span().start());
                if self.inside_node(start) {
                    node.span().start()
                } else {
                    start
                }
            }
            None => usize::from(self.source.starts_with('\u{feff}')) * '\u{feff}'.len_utf8(),
        };
        let eol = self.eol;
        self.insert(
            offset,
            format!("{text}{eol}"),
            "class_statement_insert",
            None,
        );
    }

    /// Restate an existing call: the planned classes in the positional
    /// slots, the planned named arguments in place, and every other argument
    /// byte for byte. A slot whose value already reads as planned is kept as
    /// written, so an equivalent call is left alone.
    fn replace_class_call(&mut self, call: &Node, item: &RetypePlanItem) {
        let NodeKind::Template(template) = call.kind() else {
            return;
        };
        let source = self.source;
        let text = |start: usize, end: usize| &source[start..end];
        let span = call.span();
        let name_end = template
            .arguments
            .first()
            .map_or(span.end() - 2, |argument| argument.span.start() - 1);
        let raw_name = text(span.start() + 2, name_end);

        let planned: Option<Vec<&str>> =
            (!item.classes.is_empty()).then(|| item.classes.iter().map(|id| id.trim()).collect());
        let mut classes = planned.as_deref().unwrap_or_default().iter();
        let named = named_plan(item);
        let mut named_done: BTreeSet<&str> = BTreeSet::new();
        let mut arguments: Vec<String> = Vec::new();
        let mut class_slot_end = 0usize;
        let mut slot_style = (String::new(), String::new());
        for argument in &template.arguments {
            let raw = text(argument.span.start(), argument.span.end());
            let value = text(argument.value.start(), argument.value.end());
            let name = argument
                .name
                .map(|name| text(name.start(), name.end()).trim());
            let positional = match name {
                None => true,
                Some(name) => name
                    .parse::<usize>()
                    .is_ok_and(|index| index >= 1 && name == index.to_string()),
            };
            if positional {
                if planned.is_none() {
                    arguments.push(raw.to_string());
                    continue;
                }
                let lead_end = argument
                    .name
                    .map_or(argument.value.start(), |name| name.start());
                let lead = text(argument.span.start(), lead_end.min(argument.value.start()));
                let trail = text(argument.value.end().max(lead_end), argument.span.end());
                slot_style = (lead.to_string(), trail.to_string());
                if let Some(class) = classes.next() {
                    if name.is_none() && plain_text(value) == *class {
                        arguments.push(raw.to_string());
                    } else {
                        arguments.push(format!("{lead}{class}{trail}"));
                    }
                    class_slot_end = arguments.len();
                }
                continue;
            }
            let name = name.expect("named");
            match named.get(name) {
                None => arguments.push(raw.to_string()),
                Some(_) if named_done.contains(name) => {}
                Some(planned_value) => {
                    named_done.insert(name);
                    let Some(new_value) = planned_value else {
                        continue;
                    };
                    if plain_text(value) == *new_value {
                        arguments.push(raw.to_string());
                    } else if argument.value.is_empty() {
                        let name_span = argument.name.expect("named");
                        let lead = text(argument.span.start(), name_span.start());
                        let trail = &raw[raw.trim_end().len()..];
                        arguments.push(format!("{lead}{name}={new_value}{trail}"));
                    } else {
                        let prefix = text(argument.span.start(), argument.value.start());
                        let suffix = text(argument.value.end(), argument.span.end());
                        arguments.push(format!("{prefix}{new_value}{suffix}"));
                    }
                }
            }
        }
        // The style of the last argument, for the arguments the call lacks.
        let tail = template
            .arguments
            .last()
            .map(|argument| {
                let raw = text(argument.span.start(), argument.span.end());
                raw[raw.trim_end().len()..].to_string()
            })
            .unwrap_or_default();
        let leftover: Vec<String> = classes
            .map(|class| {
                if slot_style.0.is_empty() && slot_style.1.is_empty() {
                    format!("{class}{tail}")
                } else {
                    format!("{}{class}{}", slot_style.0, slot_style.1)
                }
            })
            .collect();
        let insert_at = if class_slot_end > 0 {
            class_slot_end
        } else {
            0
        };
        for (offset, argument) in leftover.into_iter().enumerate() {
            arguments.insert(insert_at + offset, argument);
        }
        for (name, value) in &named {
            if let Some(value) = value
                && !named_done.contains(name.as_str())
            {
                arguments.push(format!("{name}={value}{tail}"));
            }
        }
        let mut replacement = format!("{{{{{raw_name}");
        for argument in &arguments {
            replacement.push('|');
            replacement.push_str(argument);
        }
        replacement.push_str("}}");
        if replacement != text(span.start(), span.end())
            && self.plan.replace(call.id(), replacement).is_ok()
        {
            self.kinds
                .insert(call.id(), ("class_statement_replace", String::new()));
        }
    }

    /// Add category links after the last kept category link, or at the end
    /// of the page after one blank line.
    fn add_links(&mut self, anchor_end: Option<usize>, names: &[&String]) -> Result<(), String> {
        let eol = self.eol;
        if let Some(anchor_end) = anchor_end {
            let mut offset = self.line_end(anchor_end);
            if self.inside_node(offset) {
                offset = anchor_end;
            }
            for name in names {
                self.insert(
                    offset,
                    format!("{eol}[[Category:{name}]]"),
                    "category_add",
                    Some((*name).clone()),
                );
            }
            return Ok(());
        }
        // The page's end as the other edits leave it decides the spacing.
        let interim = self
            .document
            .rewrite(&self.plan)
            .map_err(|error| error.to_string())?;
        let ends_with_break = interim.ends_with('\n');
        let blank_end = interim.ends_with("\n\n") || interim.ends_with("\n\r\n");
        let prefix = if interim.trim().is_empty() || blank_end {
            ""
        } else if ends_with_break {
            eol
        } else if eol == "\r\n" {
            "\r\n\r\n"
        } else {
            "\n\n"
        };
        // The page keeps its own convention for a final line break.
        let suffix = if self.source.ends_with('\n') { eol } else { "" };
        let offset = self.source.len();
        let last = names.len() - 1;
        for (index, name) in names.iter().enumerate() {
            let lead = if index == 0 { prefix } else { eol };
            let trail = if index == last { suffix } else { "" };
            self.insert(
                offset,
                format!("{lead}[[Category:{name}]]{trail}"),
                "category_add",
                Some((*name).clone()),
            );
        }
        Ok(())
    }

    fn finish(self) -> Result<(String, Vec<RetypeEdit>), String> {
        let resolved = self
            .document
            .resolve(&self.plan)
            .map_err(|error| error.to_string())?;
        let result = self
            .document
            .rewrite(&self.plan)
            .map_err(|error| error.to_string())?;
        let mut edits = Vec::with_capacity(resolved.len());
        let mut delta: isize = 0;
        for edit in resolved {
            let span = edit.span();
            let (kind, categories) = match edit.origin() {
                EditOrigin::Replace(node) => (
                    self.kinds
                        .get(node)
                        .map_or("class_statement_replace", |(kind, _)| kind),
                    Vec::new(),
                ),
                EditOrigin::RemoveLine(nodes) => {
                    let entries: Vec<&(&'static str, String)> = nodes
                        .iter()
                        .filter_map(|node| self.kinds.get(node))
                        .collect();
                    let kind = if entries.iter().any(|(kind, _)| *kind == "category_remove") {
                        "category_remove"
                    } else {
                        "category_drop_generated"
                    };
                    (kind, entries.iter().map(|(_, name)| name.clone()).collect())
                }
                EditOrigin::Insert(index) => {
                    let (kind, category) = &self.insert_kinds[*index];
                    (*kind, category.iter().cloned().collect())
                }
            };
            let result_start = (span.start() as isize + delta) as usize;
            let result_end = result_start + edit.replacement().len();
            delta += edit.replacement().len() as isize - span.len() as isize;
            edits.push(RetypeEdit {
                kind,
                categories,
                start: span.start(),
                end: span.end(),
                result_start,
                result_end,
                old: self.source[span.start()..span.end()].to_string(),
                new: edit.replacement().to_string(),
            });
        }
        Ok((result, edits))
    }
}

fn rewrite_refusal(error: &str) -> Refusal {
    Refusal::new(
        REFUSED_REWRITE_FAILED,
        format!("the edits cannot be applied as node rewrites: {error}"),
    )
}

/// How `run_article_retype` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetypeApplyMode {
    /// Write nothing; report the result.
    None,
    /// Write each changed page's result to its draft.
    Draft,
}

impl RetypeApplyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Draft => "draft",
        }
    }
}

/// The command's report.
#[derive(Debug, Clone, Serialize)]
pub struct RetypeReport {
    pub schema: &'static str,
    /// `complete`, or `not_ready` when the project cannot run the typing
    /// checks; then `reason` says why and no page is read.
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub apply_mode: &'static str,
    pub plan_path: String,
    pub plan_sha256: String,
    pub ontology_commit: Option<String>,
    pub class_template: Option<String>,
    pub page_count: usize,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub refused_count: usize,
    pub edit_count: usize,
    pub drafts_written: usize,
    pub pages: Vec<RetypePageReport>,
}

/// One page in the report.
#[derive(Debug, Clone, Serialize)]
pub struct RetypePageReport {
    pub title: String,
    /// `changed`, `unchanged` or `refused`.
    pub status: &'static str,
    pub source_path: Option<String>,
    /// `draft` or `mirror`.
    pub source_kind: Option<&'static str>,
    pub source_sha256: Option<String>,
    pub result_sha256: Option<String>,
    /// Where `--apply draft` writes the result.
    pub draft_path: Option<String>,
    pub draft_written: bool,
    pub edits: Vec<RetypeEdit>,
    /// A unified diff of the source against the result; empty when unchanged.
    pub diff: String,
    pub typing: Option<RetypeTyping>,
    pub refusals: Vec<Refusal>,
    pub notes: Vec<String>,
}

impl RetypePageReport {
    fn refused(title: String, refusal: Refusal) -> Self {
        Self {
            title,
            status: "refused",
            source_path: None,
            source_kind: None,
            source_sha256: None,
            result_sha256: None,
            draft_path: None,
            draft_written: false,
            edits: Vec::new(),
            diff: String::new(),
            typing: None,
            refusals: vec![refusal],
            notes: Vec::new(),
        }
    }
}

/// Run a plan over the project's pages: `titles` selects plan items, in
/// order, or every item when `None`.
pub fn run_article_retype(
    paths: &ResolvedPaths,
    plan_path: &Path,
    titles: Option<&[String]>,
    apply: RetypeApplyMode,
) -> Result<RetypeReport> {
    let plan_text = fs::read_to_string(plan_path)
        .with_context(|| format!("failed to read the plan {}", normalize_path(plan_path)))?;
    let plan = parse_plan(&plan_text)
        .with_context(|| format!("the plan {} is not usable", normalize_path(plan_path)))?;
    let mut report = RetypeReport {
        schema: REPORT_SCHEMA,
        status: "not_ready",
        reason: None,
        apply_mode: apply.as_str(),
        plan_path: normalize_path(plan_path),
        plan_sha256: compute_sha256(&plan_text),
        ontology_commit: None,
        class_template: None,
        page_count: 0,
        changed_count: 0,
        unchanged_count: 0,
        refused_count: 0,
        edit_count: 0,
        drafts_written: 0,
        pages: Vec::new(),
    };
    let adapter = load_site_adapter(paths)?;
    let ontology = match load_project_ontology(paths, &adapter)? {
        OntologyAvailability::NotConfigured => {
            report.reason = Some(format!(
                "the site adapter {} has no [ontology] section, so this wiki states no classes; article retype needs [ontology] with class_template, the vendored ontology, and the ontology modules",
                adapter.adapter_id
            ));
            return Ok(report);
        }
        OntologyAvailability::Unavailable { reason } => {
            report.reason = Some(reason);
            return Ok(report);
        }
        OntologyAvailability::Ready(ontology) => ontology,
    };
    report.ontology_commit = ontology.commit().map(str::to_string);
    report.class_template = ontology.class_template().map(str::to_string);
    if ontology.class_template().is_none() {
        report.reason = Some("the site adapter's [ontology] names no class_template".to_string());
        return Ok(report);
    }
    if let Some(reason) = ontology.modules_unavailable_reason() {
        report.reason = Some(reason);
        return Ok(report);
    }
    report.status = "complete";

    let items: BTreeMap<String, &RetypePlanItem> = plan
        .items
        .iter()
        .map(|item| (normalize_title(&item.title), item))
        .collect();
    let selected: Vec<String> = match titles {
        Some(titles) => dedupe(titles.iter().map(|title| normalize_title(title))),
        None => plan
            .items
            .iter()
            .map(|item| normalize_title(&item.title))
            .collect(),
    };
    for title in selected {
        let page = match items.get(&title) {
            None => RetypePageReport::refused(
                title.clone(),
                Refusal::new(
                    REFUSED_NOT_IN_PLAN,
                    format!("the plan has no item for {title:?}"),
                ),
            ),
            Some(item) => retype_page(paths, &ontology, &title, item, apply)?,
        };
        report.edit_count += page.edits.len();
        match page.status {
            "changed" => report.changed_count += 1,
            "unchanged" => report.unchanged_count += 1,
            _ => report.refused_count += 1,
        }
        report.drafts_written += usize::from(page.draft_written);
        report.pages.push(page);
    }
    report.page_count = report.pages.len();
    Ok(report)
}

/// The draft path for a title: the mirror's file name under
/// `.wikitool/drafts/`.
pub fn draft_relative_path(paths: &ResolvedPaths, title: &str) -> Result<String> {
    let mirror = title_to_relative_path(paths, title, false)?;
    let file_name = mirror.rsplit('/').next().unwrap_or(&mirror).to_string();
    let drafts = paths.state_dir.join("drafts").join(file_name);
    let relative = drafts
        .strip_prefix(&paths.project_root)
        .map(normalize_path)
        .unwrap_or_else(|_| normalize_path(&drafts));
    Ok(relative)
}

fn retype_page(
    paths: &ResolvedPaths,
    ontology: &ProjectOntology,
    title: &str,
    item: &RetypePlanItem,
    apply: RetypeApplyMode,
) -> Result<RetypePageReport> {
    if namespace_from_title(title) != Namespace::Main {
        return Ok(RetypePageReport::refused(
            title.to_string(),
            Refusal::new(
                REFUSED_NOT_AN_ARTICLE,
                format!("{title:?} is not in the article namespace"),
            ),
        ));
    }
    let draft_path = draft_relative_path(paths, title)?;
    let mirror_path = title_to_relative_path(paths, title, false)?;
    let redirect_path = title_to_relative_path(paths, title, true)?;
    let mut source = None;
    for (relative, kind) in [(&draft_path, "draft"), (&mirror_path, "mirror")] {
        match fs::read_to_string(paths.project_root.join(relative)) {
            Ok(text) => {
                source = Some((relative.clone(), kind, text));
                break;
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {relative}"));
            }
        }
    }
    let Some((source_path, source_kind, text)) = source else {
        let refusal = if paths.project_root.join(&redirect_path).exists() {
            Refusal::new(
                REFUSED_REDIRECT,
                format!("{title:?} is a redirect in the mirror at {redirect_path}"),
            )
        } else {
            Refusal::new(
                REFUSED_PAGE_MISSING,
                format!(
                    "{title:?} has no draft at {draft_path} and no mirror page at {mirror_path}"
                ),
            )
        };
        return Ok(RetypePageReport::refused(title.to_string(), refusal));
    };
    let outcome = retype_text(ontology, item, &text);
    let changed = outcome.result != text;
    let status = if !outcome.refusals.is_empty() {
        "refused"
    } else if changed {
        "changed"
    } else {
        "unchanged"
    };
    let diff = if changed {
        TextDiff::from_lines(text.as_str(), outcome.result.as_str())
            .unified_diff()
            .context_radius(3)
            .header(&format!("a/{source_path}"), &format!("b/{draft_path}"))
            .to_string()
    } else {
        String::new()
    };
    let mut draft_written = false;
    if apply == RetypeApplyMode::Draft && changed {
        atomic_write(&paths.project_root.join(&draft_path), &outcome.result)?;
        draft_written = true;
    }
    Ok(RetypePageReport {
        title: title.to_string(),
        status,
        source_path: Some(source_path),
        source_kind: Some(source_kind),
        source_sha256: Some(compute_sha256(&text)),
        result_sha256: Some(compute_sha256(&outcome.result)),
        draft_path: Some(draft_path),
        draft_written,
        edits: outcome.edits,
        diff,
        typing: Some(outcome.typing),
        refusals: outcome.refusals,
        notes: outcome.notes,
    })
}
