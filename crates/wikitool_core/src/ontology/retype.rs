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
//! contacts no wiki and records no acceptance. Every result is checked
//! before it is reported (see `guard`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mediawiki_wikitext::{
    Document, EditOrigin, LinkKind, Node, NodeId, NodeKind, ProfiledParseInput, ProtectedKind,
    RewritePlan, parse_profiled,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::TextDiff;

use crate::runtime::ResolvedPaths;
use crate::site::load_site_adapter;
use crate::support::{atomic_write, compute_sha256, normalize_path};

use super::typing::{
    OntologyAvailability, ProjectOntology, RENAMED_TERM, SkippedCheck, UNKNOWN_CLASS,
    is_wikidata_item, load_project_ontology, plain_text, template_key,
};
use super::values::wikitext_profile;

mod guard;
mod pages;
#[cfg(test)]
mod proof;
#[cfg(test)]
mod tests;

/// The plan file's schema.
pub const PLAN_SCHEMA: &str = "wikitool.article-retype-plan.v1";
/// The report's schema.
pub const REPORT_SCHEMA: &str = "wikitool.article-retype.v1";

/// Classes `Module:Infobox`'s `categoryClass` looks at, the class and its
/// superclasses, before it gives up.
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
    /// Where a link or call sits that cannot be edited, or where a new one
    /// would land: `template`, `parameter`, `link`, `includeonly`,
    /// `noinclude`, `onlyinclude`, `gallery`, `ref`, `math`, `score`,
    /// `templatedata`, `comment`, `nowiki`, `pre`, `source`,
    /// `syntaxhighlight`, `multiple` for more than one class call, or
    /// `unreadable` for a link whose category cannot be read.
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
pub const REFUSED_RESULT_CHECK_FAILED: &str = "result_check_failed";
pub const REFUSED_INVALID_TITLE: &str = "invalid_title";
pub const REFUSED_AMBIGUOUS_DRAFT: &str = "ambiguous_draft";
pub const REFUSED_DUPLICATE_TARGET: &str = "duplicate_target";

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
    /// The normalized category name, or the raw text when `unreadable`.
    name: String,
    /// The name holds markup, such as a template call, so which category
    /// the link files the page in cannot be read from the text.
    unreadable: bool,
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
    let class_refusals = planned_class_refusals(
        ontology,
        item,
        class_key.trim_start_matches("Template:"),
    );
    if !class_refusals.is_empty() {
        return PageRetype::refused(source, ontology, class_refusals);
    }
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
        eol: majority_line_break(source),
        plan: RewritePlan::new(),
        insertions: Vec::new(),
        kinds: BTreeMap::new(),
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
    // The classes the class statement states after the edits; `None` when
    // the page gets no class statement.
    let mut stated: Option<Vec<String>> = None;
    match class_calls.as_slice() {
        [] => {
            if !item.classes.is_empty() {
                if let Some(reason) = typed_infobox_states(ontology, &statements, item) {
                    editor.notes.push(reason);
                    if item.drop_generated {
                        editor.notes.push(
                            "drop_generated removed nothing: the page gets no class statement, and its typed infobox files the page in categories of its own".to_string(),
                        );
                    }
                } else {
                    editor.insert_class_call(ontology, &template_name, item);
                    stated = Some(
                        item.classes
                            .iter()
                            .map(|id| id.trim().to_string())
                            .collect(),
                    );
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
                stated = Some(if item.classes.is_empty() {
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
                });
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
    let generated: Vec<String> = match &stated {
        Some(classes) => {
            let (generated, notes) = generated_categories(ontology, classes);
            if item.drop_generated {
                editor.notes.extend(notes);
            }
            generated
        }
        None => Vec::new(),
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
        if link.unreadable && !relevant.is_empty() {
            let mut refusal = Refusal::new(
                REFUSED_CATEGORY_NOT_EDITABLE,
                format!(
                    "the link {:?} names its category with markup, so it may file the page in a category the plan edits",
                    &source[link.start..link.end]
                ),
            );
            refusal.context = Some("unreadable");
            refusal.start = Some(link.start);
            refusal.end = Some(link.end);
            editor.refusals.push(refusal);
            continue;
        }
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
        .filter(|link| link.context.is_none() && link.node.is_some() && !link.unreadable)
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
        editor.add_links(anchor.map(|link| link.end), &to_add);
        if !editor.refusals.is_empty() {
            return PageRetype::refused(source, ontology, editor.refusals);
        }
    }

    let mut notes = std::mem::take(&mut editor.notes);
    let (result, edits) = match editor.finish(&mut notes) {
        Ok(done) => done,
        Err(error) => {
            return PageRetype::refused(source, ontology, vec![rewrite_refusal(&error)]);
        }
    };
    if let Err(error) = guard::check_result(source, &result, &edits, &class_key) {
        return PageRetype::refused(
            source,
            ontology,
            vec![Refusal::new(
                REFUSED_RESULT_CHECK_FAILED,
                format!("the result fails its check, so the page is left as it is: {error}"),
            )],
        );
    }
    let typing = typing_of(ontology, &result);
    PageRetype {
        result,
        edits,
        refusals: Vec::new(),
        notes,
        typing,
    }
}

/// The planned class ids the typing checks would report, with the codes and
/// wording of `article lint`'s class-statement check: a renamed id, and an
/// id that is not a class. They are checked whether or not the page gets a
/// class statement.
fn planned_class_refusals(
    ontology: &ProjectOntology,
    item: &RetypePlanItem,
    template_name: &str,
) -> Vec<Refusal> {
    let mut out = Vec::new();
    for id in &item.classes {
        let id = id.trim();
        let current = ontology.vocabulary.current(id);
        if current != id {
            let mut refusal = Refusal::new(
                RENAMED_TERM,
                format!(
                    "the plan states {{{{{template_name}|{id}}}}}, which the ontology renamed to {current}"
                ),
            );
            refusal.term = Some(id.to_string());
            out.push(refusal);
            continue;
        }
        let message = match ontology.vocabulary.find(id) {
            Some(term) if term.section == "classes" => continue,
            Some(term) => format!(
                "the plan states {{{{{template_name}|{id}}}}}, but {id} is among the {} of the ontology, not its classes",
                term.section
            ),
            None => format!(
                "the plan states {{{{{template_name}|{id}}}}}, but {id} is not a class in Module:Ontology/data or Module:Ontology/local"
            ),
        };
        let mut refusal = Refusal::new(UNKNOWN_CLASS, message);
        refusal.term = Some(id.to_string());
        out.push(refusal);
    }
    out
}

/// `\r\n` when most of the page's line breaks are, else `\n`.
fn majority_line_break(source: &str) -> &'static str {
    let breaks = source.matches('\n').count();
    let crlf = source.matches("\r\n").count();
    if crlf * 2 > breaks { "\r\n" } else { "\n" }
}

fn dedupe(names: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    names.filter(|name| seen.insert(name.clone())).collect()
}

/// Characters a class id may not hold: wikitext markup, the argument
/// separators, and line breaks.
const ID_FORBIDDEN: &[char] = &['|', '{', '}', '[', ']', '<', '>', '=', '\n', '\r'];
/// Characters a field value may not hold: wikitext markup (links, calls,
/// tags and comments), the argument and value separators, and line breaks.
/// `=` is allowed, since only the first `=` of a named argument splits it.
const VALUE_FORBIDDEN: &[char] = &['|', '{', '}', '[', ']', '<', '>', ';', '\n', '\r'];
const CATEGORY_FORBIDDEN: &[char] = &['[', ']', '{', '}', '|', '#', '<', '>', '\n', '\r'];

/// Text that holds markup the command must not write: a forbidden
/// character, a control character, or a signature (`~~~`), which MediaWiki
/// replaces when the page is saved.
fn holds_markup(text: &str, forbidden: &[char]) -> bool {
    text.contains(forbidden) || text.chars().any(char::is_control) || text.contains("~~~")
}

fn validate_item(item: &RetypePlanItem) -> Vec<Refusal> {
    let mut out = Vec::new();
    let mut invalid = |message: String| out.push(Refusal::new(REFUSED_INVALID_PLAN, message));
    let mut classes = BTreeSet::new();
    for id in &item.classes {
        let id = id.trim();
        if id.is_empty() || holds_markup(id, ID_FORBIDDEN) {
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
            if value.trim().is_empty() || holds_markup(value, VALUE_FORBIDDEN) {
                invalid(format!(
                    "the {name} value {value:?} is empty or holds a separator or markup"
                ));
            }
        }
    }
    if let Some(wikidata) = &item.wikidata
        && !wikidata.trim().is_empty()
        && !is_wikidata_item(wikidata.trim())
    {
        invalid(format!(
            "wikidata {wikidata:?} is not a Wikidata item id such as Q42"
        ));
    }
    for name in item.remove_categories.iter().chain(&item.add_categories) {
        if normalize_category(name).is_empty()
            || holds_markup(name, CATEGORY_FORBIDDEN)
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
/// `instanceOf` computes them. It reads the positional classes up to the
/// first empty one and skips an id that is not a class. Each class files the
/// page in the category of its `categoryClass`: the class itself when
/// `Module:Ontology/display` lists it in `classes`, else its superclass, and
/// so on, through `subClassOf` read as one id, for at most fifteen classes.
/// Like the module, the walk follows no renames (`Module:Ontology/data`
/// holds none) and stops at a `subClassOf` that lists several classes. When
/// that category is marked `small`, the page also goes in the category of
/// the nearest superclass whose category is not small. The notes say which
/// classes file the page in no category, and why.
fn generated_categories(
    ontology: &ProjectOntology,
    classes: &[String],
) -> (Vec<String>, Vec<String>) {
    let vocabulary = &ontology.vocabulary;
    let mut out = Vec::new();
    let mut notes = Vec::new();
    let mut add = |class: &str| {
        if let Some(name) = vocabulary
            .display_class(class)
            .and_then(|entry| entry.get("category"))
            .and_then(Value::as_str)
        {
            let name = normalize_category(name);
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
    };
    for id in classes.iter().take_while(|id| !id.trim().is_empty()) {
        let id = id.trim();
        if vocabulary.term("classes", id).is_none() {
            continue;
        }
        match category_class(ontology, id, false) {
            CategoryWalk::Found(owner) => {
                add(&owner);
                let small = vocabulary.display_class(&owner).is_some_and(is_small);
                if small {
                    let parent = vocabulary
                        .term("classes", &owner)
                        .and_then(|term| term.entry.get("subClassOf"))
                        .and_then(Value::as_str);
                    if let Some(parent) = parent
                        && let CategoryWalk::Found(larger) = category_class(ontology, parent, true)
                    {
                        add(&larger);
                    }
                }
            }
            CategoryWalk::SeveralParents(at) => notes.push(format!(
                "the class statement files the page in no category for {id}: {at} lists several classes in subClassOf, and Module:Infobox follows only one"
            )),
            CategoryWalk::NotFound => notes.push(format!(
                "the class statement files the page in no category for {id}: neither it nor a superclass has a category in Module:Ontology/display"
            )),
        }
    }
    (out, notes)
}

fn is_small(entry: &serde_json::Map<String, Value>) -> bool {
    entry.get("small").and_then(Value::as_bool) == Some(true)
}

enum CategoryWalk {
    Found(String),
    /// The walk reached a class whose `subClassOf` is a list.
    SeveralParents(String),
    NotFound,
}

/// `Module:Infobox`'s `categoryClass`: the class, or the nearest superclass,
/// that `Module:Ontology/display` gives a category; with `skip_small`, one
/// whose category is not small. Superclasses are read with `Ontology.find`,
/// from any section, shared before local.
fn category_class(ontology: &ProjectOntology, id: &str, skip_small: bool) -> CategoryWalk {
    let mut current = id.to_string();
    for _ in 0..MAX_CATEGORY_HOPS {
        if let Some(entry) = ontology.vocabulary.display_class(&current)
            && !(skip_small && is_small(entry))
        {
            return CategoryWalk::Found(current);
        }
        let parent = ontology
            .vocabulary
            .find(&current)
            .and_then(|term| term.entry.get("subClassOf"));
        match parent {
            Some(Value::String(parent)) => current = parent.clone(),
            Some(Value::Array(_)) => return CategoryWalk::SeveralParents(current),
            _ => return CategoryWalk::NotFound,
        }
    }
    CategoryWalk::NotFound
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

/// The category a category link files the page in, whether it has a sort
/// key, and whether the name holds markup; `None` for a link to the
/// category page (`[[:Category:…]]`) or another kind of link.
fn link_category(document: &Document<'_>, node: &Node) -> Option<(String, bool, bool)> {
    let NodeKind::InternalLink(link) = node.kind() else {
        return None;
    };
    if link.kind != LinkKind::Category || link.leading_colon {
        return None;
    }
    let title = document.text(link.title);
    let (_, name) = title.split_once(':')?;
    let unreadable = name.contains(['{', '}', '<', '>', '[', ']']);
    let name = if unreadable {
        name.trim().to_string()
    } else {
        normalize_title(name)
    };
    Some((name, !link.components.is_empty(), unreadable))
}

/// Every category link on the page: the parsed link nodes, and the links in
/// comments and literal regions, which the parser leaves unread and which
/// are read here from the text, so nothing there can hide one.
fn category_links(document: &Document<'_>) -> Vec<CategoryLink> {
    let profile = wikitext_profile(document.source());
    let mut prefixes: Vec<String> = profile
        .category_namespace_aliases
        .iter()
        .map(|alias| alias.to_lowercase())
        .collect();
    prefixes.push("category".to_string());
    let mut out = Vec::new();
    for node in document.nodes() {
        if let Some((name, sort_key, unreadable)) = link_category(document, node) {
            out.push(CategoryLink {
                node: Some(node.id()),
                name,
                unreadable,
                start: node.span().start(),
                end: node.span().end(),
                sort_key,
                context: node_context(document, node),
            });
        }
        if let NodeKind::Protected(region) = node.kind() {
            text_category_links(
                document.text(region.inner),
                region.inner.start(),
                protected_name(region.kind),
                &prefixes,
                &mut out,
            );
        }
    }
    out
}

/// The category links in unparsed text, read lexically: `[[`, spaces, a
/// category namespace name, `:`, and the name up to `|`, `]]` or the end of
/// the line. A link to the category page (`[[:Category:…]]`) is not one.
fn text_category_links(
    text: &str,
    offset: usize,
    context: &'static str,
    prefixes: &[String],
    out: &mut Vec<CategoryLink>,
) {
    let mut search = 0;
    while let Some(found) = text[search..].find("[[") {
        let start = search + found;
        search = start + 2;
        let rest = &text[start + 2..];
        let target_end = rest
            .find(['|', '\n'])
            .into_iter()
            .chain(rest.find("]]"))
            .min()
            .unwrap_or(rest.len());
        let target = rest[..target_end].trim_start();
        let Some((namespace, name)) = target.split_once(':') else {
            continue;
        };
        if !prefixes.contains(&namespace.trim().to_lowercase()) {
            continue;
        }
        let link_end = rest
            .find("]]")
            .map_or(start + 2 + target_end, |close| start + 2 + close + 2);
        let unreadable = name.contains(['{', '}', '<', '>', '[']);
        out.push(CategoryLink {
            node: None,
            name: if unreadable {
                name.trim().to_string()
            } else {
                normalize_title(name)
            },
            unreadable,
            start: offset + start,
            end: offset + link_end.max(start + 2),
            sort_key: rest[target_end..].starts_with('|'),
            context: Some(context),
        });
    }
}

/// The plan for one page as it is built, and what each edit is for.
struct Editor<'d, 's> {
    document: &'d Document<'s>,
    source: &'s str,
    /// The page's usual line break, for a line with none of its own.
    eol: &'static str,
    /// Replacements and line removals.
    plan: RewritePlan,
    /// Insertions, in the order they were planned, added to the plan last.
    insertions: Vec<Insertion>,
    kinds: BTreeMap<NodeId, (&'static str, String)>,
    refusals: Vec<Refusal>,
    notes: Vec<String>,
}

struct Insertion {
    offset: usize,
    text: String,
    kind: &'static str,
    category: Option<String>,
}

impl Editor<'_, '_> {
    fn insert(
        &mut self,
        offset: usize,
        text: String,
        kind: &'static str,
        category: Option<String>,
    ) {
        self.insertions.push(Insertion {
            offset,
            text,
            kind,
            category,
        });
    }

    fn remove_link(&mut self, node: NodeId, kind: &'static str, category: &str) {
        if self.plan.remove_line(node).is_ok() {
            self.kinds.insert(node, (kind, category.to_string()));
        }
    }

    /// What a byte offset falls inside, when new text may not go there: a
    /// node (strictly inside it), or a tag region (strictly inside it, or at
    /// the end of one that never closes).
    fn blocked_at(&self, offset: usize) -> Option<&'static str> {
        if let Some(node) = self
            .document
            .nodes()
            .iter()
            .find(|node| node.span().start() < offset && offset < node.span().end())
        {
            return Some(match node.kind() {
                NodeKind::Template(_) => "template",
                NodeKind::Parameter(_) => "parameter",
                NodeKind::InternalLink(_) => "link",
                NodeKind::Protected(region) => protected_name(region.kind),
            });
        }
        self.document
            .tag_regions()
            .iter()
            .find(|region| {
                region.span.start() < offset
                    && (offset < region.span.end()
                        || (!region.closed && offset == region.span.end()))
            })
            .map(|region| region.tag.name())
    }

    /// The first of `offsets` where new text may go, or a refusal naming
    /// where the last one falls.
    fn place(
        &mut self,
        offsets: &[usize],
        code: &'static str,
        what: &str,
    ) -> Option<usize> {
        let mut context = None;
        for offset in offsets {
            match self.blocked_at(*offset) {
                None => return Some(*offset),
                Some(found) => context = Some(found),
            }
        }
        let context = context.unwrap_or("template");
        let mut refusal = Refusal::new(
            code,
            format!("{what} would go inside a {context}, so the page is left as it is"),
        );
        refusal.context = Some(context);
        refusal.start = offsets.last().copied();
        refusal.end = offsets.last().copied();
        self.refusals.push(refusal);
        None
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

    /// The line break of the line holding `offset`; for the last line, which
    /// has none, the one before it; else the page's usual one.
    fn line_break_at(&self, offset: usize) -> &'static str {
        let bytes = self.source.as_bytes();
        let ending = |newline: usize| {
            if newline > 0 && bytes[newline - 1] == b'\r' {
                "\r\n"
            } else {
                "\n"
            }
        };
        if let Some(index) = bytes[offset..].iter().position(|byte| *byte == b'\n') {
            return ending(offset + index);
        }
        match bytes[..offset].iter().rposition(|byte| *byte == b'\n') {
            Some(newline) => ending(newline),
            None => self.eol,
        }
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
        let what = format!("the new {{{{{template_name}}}}} call");
        if let Some((node, _)) = short_description {
            let end = node.span().end();
            let eol = self.line_break_at(end);
            let offsets = [self.line_end(end), end];
            if let Some(offset) = self.place(&offsets, REFUSED_CLASS_STATEMENT_NOT_EDITABLE, &what)
            {
                self.insert(
                    offset,
                    format!("{eol}{text}"),
                    "class_statement_insert",
                    None,
                );
            }
            return;
        }
        let infobox = templates.iter().find(|(_, name)| {
            template_key(name).is_some_and(|key| {
                ontology.is_typed_infobox(&key) || key.starts_with("Template:Infobox")
            })
        });
        let (offsets, eol) = match infobox {
            Some((node, _)) => {
                let start = node.span().start();
                (vec![self.line_start(start), start], self.line_break_at(start))
            }
            None => {
                let top = usize::from(self.source.starts_with('\u{feff}')) * '\u{feff}'.len_utf8();
                (vec![top], self.line_break_at(top))
            }
        };
        if let Some(offset) = self.place(&offsets, REFUSED_CLASS_STATEMENT_NOT_EDITABLE, &what) {
            self.insert(
                offset,
                format!("{text}{eol}"),
                "class_statement_insert",
                None,
            );
        }
    }

    /// Restate an existing call: the planned classes in the positional
    /// slots, the planned named arguments in place, and every other argument
    /// byte for byte. A slot whose value already reads as planned, `1=a` as
    /// much as `a`, is kept as written, so an equivalent call is left alone.
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
                    // An explicit `1=` slot that already reads as planned is
                    // kept as written too.
                    if plain_text(value) == *class {
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
    fn add_links(&mut self, anchor_end: Option<usize>, names: &[&String]) {
        let what = "a new category link";
        if let Some(anchor_end) = anchor_end {
            let eol = self.line_break_at(anchor_end);
            let offsets = [self.line_end(anchor_end), anchor_end];
            let Some(offset) = self.place(&offsets, REFUSED_CATEGORY_NOT_EDITABLE, what) else {
                return;
            };
            for name in names {
                self.insert(
                    offset,
                    format!("{eol}[[Category:{name}]]"),
                    "category_add",
                    Some((*name).clone()),
                );
            }
            return;
        }
        let end = self.source.len();
        let Some(offset) = self.place(&[end], REFUSED_CATEGORY_NOT_EDITABLE, what) else {
            return;
        };
        // The page's end as the other edits leave it decides the spacing.
        let interim = match self.document.rewrite(&self.planned()) {
            Ok(interim) => interim,
            Err(error) => {
                self.refusals.push(rewrite_refusal(&error.to_string()));
                return;
            }
        };
        let eol = self.line_break_at(end);
        let ends_with_break = interim.ends_with('\n');
        let blank_end = interim.ends_with("\n\n") || interim.ends_with("\n\r\n");
        let blank = format!("{eol}{eol}");
        let prefix = if interim.trim().is_empty() || blank_end {
            ""
        } else if ends_with_break {
            eol
        } else {
            blank.as_str()
        };
        // The page keeps its own final line break, or its lack of one.
        let suffix = if self.source.ends_with("\r\n") {
            "\r\n"
        } else if self.source.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let last = names.len() - 1;
        for (index, name) in names.iter().enumerate() {
            let lead = if index == 0 { prefix } else { eol };
            let trail = if index == last { suffix } else { "" };
            let text = format!("{lead}[[Category:{name}]]{trail}");
            self.insert(offset, text, "category_add", Some((*name).clone()));
        }
    }

    /// The plan with the insertions added, in the order they were planned.
    fn planned(&self) -> RewritePlan {
        let mut plan = self.plan.clone();
        for insertion in &self.insertions {
            plan.insert(insertion.offset, insertion.text.clone());
        }
        plan
    }

    /// The result and its edits. When the edits would end a page that has no
    /// final line break with one, the line break goes too, so the page keeps
    /// its final-line-break state.
    fn finish(self, notes: &mut Vec<String>) -> Result<(String, Vec<RetypeEdit>), String> {
        let plan = self.planned();
        let resolved = self
            .document
            .resolve(&plan)
            .map_err(|error| error.to_string())?;
        let mut edits = Vec::with_capacity(resolved.len());
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
                    let insertion = &self.insertions[*index];
                    (insertion.kind, insertion.category.iter().cloned().collect())
                }
                _ => return Err("the plan resolved to an edit of an unknown kind".to_string()),
            };
            edits.push(RetypeEdit {
                kind,
                categories,
                start: span.start(),
                end: span.end(),
                result_start: 0,
                result_end: 0,
                old: self.source[span.start()..span.end()].to_string(),
                new: edit.replacement().to_string(),
            });
        }
        let source = self.source;
        let mut result = apply_edits(source, &mut edits);
        if !source.ends_with('\n') {
            // Each pass drops one line break; a page cannot need more passes
            // than it has lines.
            for _ in 0..=source.matches('\n').count() {
                if !result.ends_with('\n') || !drop_final_break(source, &result, &mut edits) {
                    break;
                }
                result = apply_edits(source, &mut edits);
            }
        }
        if !result.is_empty() && result.ends_with('\n') != source.ends_with('\n') {
            notes.push(if source.ends_with('\n') {
                "the result has no final line break, though the page has one".to_string()
            } else {
                "the result ends with a line break, though the page does not".to_string()
            });
        }
        Ok((result, edits))
    }
}

/// The result of the edits, with each edit's place in it filled in.
fn apply_edits(source: &str, edits: &mut [RetypeEdit]) -> String {
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0;
    for edit in edits.iter_mut() {
        out.push_str(&source[cursor..edit.start]);
        edit.result_start = out.len();
        out.push_str(&edit.new);
        edit.result_end = out.len();
        cursor = edit.end;
    }
    out.push_str(&source[cursor..]);
    out
}

/// Take the result's final line break away from it: trim it from the edit
/// that wrote it, or extend the removal that follows it in the source over
/// it. Whether it could.
fn drop_final_break(source: &str, result: &str, edits: &mut [RetypeEdit]) -> bool {
    let width = if result.ends_with("\r\n") { 2 } else { 1 };
    let at = result.len() - width;
    if let Some(edit) = edits
        .iter_mut()
        .find(|edit| edit.result_start <= at && at < edit.result_end)
    {
        if edit.result_end != result.len() || edit.new.len() < width {
            return false;
        }
        let keep = edit.new.len() - width;
        edit.new.truncate(keep);
        return true;
    }
    // The break is page text: find where, then the edit right after it.
    let mut source_cursor = 0;
    let mut result_cursor = 0;
    let mut break_start = None;
    for edit in edits.iter() {
        let gap = edit.start - source_cursor;
        if (result_cursor..result_cursor + gap).contains(&at) {
            break_start = Some(source_cursor + (at - result_cursor));
            break;
        }
        source_cursor = edit.end;
        result_cursor += gap + edit.new.len();
    }
    let Some(break_start) = break_start else {
        return false;
    };
    match edits
        .iter_mut()
        .find(|edit| edit.start == break_start + width)
    {
        Some(edit) if edit.new.is_empty() => {
            edit.start = break_start;
            edit.old = source[edit.start..edit.end].to_string();
            true
        }
        _ => false,
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
