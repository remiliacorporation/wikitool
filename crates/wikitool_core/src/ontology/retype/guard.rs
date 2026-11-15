//! The check every result passes before it is reported: it parses, every
//! node and tag region outside the edits is the same, each edit holds only
//! what it plans (one class call, or one category link, with nothing new
//! inside), and the page's category links change only by the planned
//! removals and additions. A result that fails is refused, never written.

use std::collections::BTreeMap;

use mediawiki_wikitext::{Document, NodeKind, ProfiledParseInput, parse_profiled};

use crate::ontology::typing::template_key;
use crate::ontology::values::wikitext_profile;

use super::{RetypeEdit, category_links};

/// A node or tag region, as compared across the source and the result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Shape {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) depth: usize,
    pub(super) kind: &'static str,
    pub(super) text: String,
}

pub(super) fn parse(text: &str) -> Result<Document<'_>, String> {
    let profile = wikitext_profile(text);
    parse_profiled(ProfiledParseInput {
        source: &profile,
        wikitext: text,
    })
    .map_err(|error| error.to_string())
}

/// Check a result against its source and edits.
pub(super) fn check_result(
    source: &str,
    result: &str,
    edits: &[RetypeEdit],
    class_key: &str,
) -> Result<(), String> {
    if edits.is_empty() {
        return Ok(());
    }
    let before = parse(source).map_err(|error| format!("the source does not parse: {error}"))?;
    let after = parse(result).map_err(|error| format!("the result does not parse: {error}"))?;
    outside_edits_match(&before, &after, edits)?;
    let mut expected = category_counts(&before);
    for edit in edits {
        check_edit(edit, class_key)?;
        for name in &edit.categories {
            let count = expected.entry(name.clone()).or_default();
            match edit.kind {
                "category_add" => *count += 1,
                _ => *count -= 1,
            }
        }
    }
    expected.retain(|_, count| *count != 0);
    let found = category_counts(&after);
    if expected != found {
        return Err(format!(
            "the result's category links are {found:?}, not the planned {expected:?}"
        ));
    }
    Ok(())
}

fn category_counts(document: &Document<'_>) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    for link in category_links(document) {
        *out.entry(link.name).or_insert(0) += 1;
    }
    out
}

/// What one edit may hold.
fn check_edit(edit: &RetypeEdit, class_key: &str) -> Result<(), String> {
    match edit.kind {
        "category_remove" | "category_drop_generated" => {
            if !edit.new.is_empty() {
                return Err(format!("a removal at {} writes text", edit.start));
            }
        }
        "category_add" => {
            let text = edit.new.trim_matches(['\r', '\n']);
            let document =
                parse(text).map_err(|error| format!("an added link does not parse: {error}"))?;
            let links = category_links(&document);
            let [link] = links.as_slice() else {
                return Err(format!("the text added at {} is not one link", edit.start));
            };
            if document.nodes().len() != 1
                || !document.tag_regions().is_empty()
                || link.node.is_none()
                || link.start != 0
                || link.end != text.len()
                || edit.categories != [link.name.clone()]
            {
                return Err(format!(
                    "the text added at {} is not one plain category link",
                    edit.start
                ));
            }
        }
        _ => {
            let new = class_call_parts(edit.new.trim_matches(['\r', '\n']), class_key)
                .map_err(|error| format!("the class statement at {}: {error}", edit.start))?;
            let old = if edit.old.is_empty() {
                Vec::new()
            } else {
                class_call_parts(&edit.old, class_key).map_err(|error| {
                    format!("the replaced class statement at {}: {error}", edit.start)
                })?
            };
            let mut available: BTreeMap<(&str, &str), usize> = BTreeMap::new();
            for part in &old {
                *available.entry((part.0, part.1.as_str())).or_default() += 1;
            }
            for part in &new {
                match available.get_mut(&(part.0, part.1.as_str())) {
                    Some(count) if *count > 0 => *count -= 1,
                    _ => {
                        return Err(format!(
                            "the class statement at {} holds a new {} {:?}",
                            edit.start, part.0, part.1
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// The nodes and tag regions inside one class call, which must be the only
/// top-level thing in its text.
fn class_call_parts(text: &str, class_key: &str) -> Result<Vec<(&'static str, String)>, String> {
    let document = parse(text).map_err(|error| format!("it does not parse: {error}"))?;
    let top: Vec<_> = document
        .nodes()
        .iter()
        .filter(|node| node.parent().is_none())
        .collect();
    let [call] = top.as_slice() else {
        return Err(format!(
            "it holds {} top-level nodes, not one call",
            top.len()
        ));
    };
    let is_class_call = match call.kind() {
        NodeKind::Template(template) => {
            template_key(document.text(template.name)).as_deref() == Some(class_key)
        }
        _ => false,
    };
    if !is_class_call || call.span().start() != 0 || call.span().end() != text.len() {
        return Err("it is not one class-template call".to_string());
    }
    let mut out: Vec<(&'static str, String)> = document
        .nodes()
        .iter()
        .filter(|node| node.id() != call.id())
        .map(|node| {
            (
                node_kind(node.kind()),
                document.text(node.span()).to_string(),
            )
        })
        .collect();
    out.extend(
        document
            .tag_regions()
            .iter()
            .map(|region| (region.tag.name(), document.text(region.span).to_string())),
    );
    Ok(out)
}

fn node_kind(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Template(_) => "template",
        NodeKind::Parameter(_) => "parameter",
        NodeKind::InternalLink(_) => "link",
        NodeKind::Protected(_) => "protected",
    }
}

/// Every node and tag region that no edit touches is in the result too, at
/// its shifted offset, with the same depth and text, and nothing else is.
pub(super) fn outside_edits_match(
    before: &Document<'_>,
    after: &Document<'_>,
    edits: &[RetypeEdit],
) -> Result<(), String> {
    let source_spans: Vec<(usize, usize)> =
        edits.iter().map(|edit| (edit.start, edit.end)).collect();
    let result_spans: Vec<(usize, usize)> = edits
        .iter()
        .map(|edit| (edit.result_start, edit.result_end))
        .collect();
    let shift = |offset: usize| -> usize {
        let mut delta: isize = 0;
        for edit in edits {
            // An insertion at a node's start goes before the node.
            if edit.end <= offset {
                delta += edit.new.len() as isize - (edit.end - edit.start) as isize;
            }
        }
        (offset as isize + delta) as usize
    };
    let mut outside_source: Vec<Shape> = shapes(before, &source_spans)
        .into_iter()
        .map(|shape| Shape {
            start: shift(shape.start),
            end: shift(shape.start) + (shape.end - shape.start),
            ..shape
        })
        .collect();
    let mut outside_result = shapes(after, &result_spans);
    outside_source.sort();
    outside_result.sort();
    if outside_source != outside_result {
        let lost: Vec<_> = outside_source
            .iter()
            .filter(|shape| !outside_result.contains(shape))
            .take(3)
            .collect();
        let gained: Vec<_> = outside_result
            .iter()
            .filter(|shape| !outside_source.contains(shape))
            .take(3)
            .collect();
        return Err(format!(
            "nodes outside the edits differ: lost {lost:?}, gained {gained:?}"
        ));
    }
    Ok(())
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
            kind: node_kind(node.kind()),
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
