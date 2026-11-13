//! Heading and spacing mechanics from a site's style guide. Every rule here is
//! off unless the adapter's `[lint]` table enables it.
//!
//! The rules read the parsed document: headings from its sections, templates,
//! refs and parser tags by their spans, and comments and literal tags (`<pre>`,
//! `<nowiki>`, `<syntaxhighlight>` and similar) as regions to skip. Internal
//! links inside a heading are not checked, because the document model records
//! links without positions.

use crate::article_lint::document::{HeadingRecord, LineRecord, ParsedArticleDocument};
use crate::article_lint::fix::TextEdit;
use crate::article_lint::model::{ArticleLintIssue, ArticleLintSeverity, TextSpan};
use crate::content_store::parsing::parse_wikilink;
use crate::filesystem::Namespace;
use crate::site::{DEFAULT_MAX_LEAD_PARAGRAPHS, LintAdvisoryLevel, LintRuleLevel};

use super::common::safe_fix_for_edit;
use super::{IssueMatch, SafeFixEdit};
use crate::article_lint::resources::LoadedResources;

const LEADING_ARTICLES: &[&str] = &["The", "A", "An"];
const TRAILING_PUNCTUATION: &[char] = &['.', ',', ':', ';', '!', '?'];
/// Abbreviations whose period is part of a name, as in "Remilia Corporation Inc.".
const NAME_ABBREVIATIONS: &[&str] = &["Inc.", "Ltd.", "Co.", "Corp.", "Jr.", "Sr.", "Bros."];

fn rule_severity(level: LintRuleLevel) -> Option<ArticleLintSeverity> {
    match level {
        LintRuleLevel::Ignore => None,
        LintRuleLevel::Warning => Some(ArticleLintSeverity::Warning),
        LintRuleLevel::Error => Some(ArticleLintSeverity::Error),
    }
}

fn advisory_severity(level: LintAdvisoryLevel) -> Option<ArticleLintSeverity> {
    match level {
        LintAdvisoryLevel::Ignore => None,
        LintAdvisoryLevel::Warning => Some(ArticleLintSeverity::Warning),
    }
}

fn issue(
    rule_id: &str,
    severity: ArticleLintSeverity,
    message: String,
    span: Option<TextSpan>,
    evidence: String,
    remediation: String,
) -> IssueMatch {
    IssueMatch {
        issue: ArticleLintIssue {
            rule_id: rule_id.to_string(),
            severity,
            message,
            span,
            evidence: Some(evidence),
            suggested_remediation: Some(remediation),
            suggested_fixes: Vec::new(),
        },
        safe_fixes: Vec::new(),
    }
}

/// Headings MediaWiki renders: those that do not start inside a comment or a
/// literal tag.
fn rendered_headings(document: &ParsedArticleDocument) -> impl Iterator<Item = &HeadingRecord> {
    document
        .sections
        .iter()
        .filter_map(|section| section.heading.as_ref())
        .filter(|heading| !document.in_literal_region(heading.start))
}

pub(super) fn lint_heading_mechanics(
    document: &ParsedArticleDocument,
    resources: &LoadedResources,
    matches: &mut Vec<IssueMatch>,
) {
    let lint = &resources.adapter.lint;
    let leading_article = advisory_severity(lint.heading_leading_article);
    let trailing_punctuation = advisory_severity(lint.heading_trailing_punctuation);
    let markup = rule_severity(lint.heading_markup);
    let repeats_title = advisory_severity(lint.heading_repeats_title);
    let title_words = title_words(document);

    for heading in rendered_headings(document) {
        let span = document.span_for_range(heading.start, heading.end);
        if let Some(severity) = leading_article
            && let Some(article) = leading_article_of(&heading.text)
        {
            matches.push(issue(
                "style.heading_leading_article",
                severity,
                format!(
                    "Heading begins with \"{article}\"; drop the leading article unless it is part of a name."
                ),
                span.clone(),
                heading.text.clone(),
                format!(
                    "Start the heading with its subject (\"{article} early years\" becomes \"Early years\"). Keep \"{article}\" when it belongs to a name, such as a band, a work or an organization."
                ),
            ));
        }
        if let Some(severity) = trailing_punctuation
            && let Some(mark) = trailing_punctuation_of(&heading.text)
        {
            matches.push(issue(
                "style.heading_trailing_punctuation",
                severity,
                format!(
                    "Heading ends in \"{mark}\"; remove it unless it belongs to a name or a quoted title."
                ),
                span.clone(),
                heading.text.clone(),
                "Delete the final punctuation mark. A title that ends in \"?\" or \"!\" keeps it inside its quotation marks or italics.".to_string(),
            ));
        }
        if let Some(severity) = markup {
            let kinds = heading_markup_kinds(document, heading);
            if !kinds.is_empty() {
                matches.push(issue(
                    "style.heading_markup",
                    severity,
                    format!(
                        "Heading contains {}; move it into the section text.",
                        kinds.join(" and ")
                    ),
                    span.clone(),
                    heading.text.clone(),
                    "Headings become link anchors and table-of-contents entries, which markup breaks. Put a citation on the section's first sentence and write the heading as plain text.".to_string(),
                ));
            }
        }
        if let Some(severity) = repeats_title
            && heading_repeats_title(&heading.text, &title_words)
        {
            matches.push(issue(
                "style.heading_repeats_title",
                severity,
                "Heading repeats the article title; the reader already knows the subject."
                    .to_string(),
                span.clone(),
                heading.text.clone(),
                "Shorten the heading (\"History of the subject\" becomes \"History\"). Keep the words when the heading names a different subject that contains the title, such as a sequel or an organization.".to_string(),
            ));
        }
    }
}

/// The leading "The", "A" or "An" of a heading of two or more words. A heading
/// that opens with a quotation mark or wikitext italics names a work and is
/// left alone.
fn leading_article_of(heading: &str) -> Option<&'static str> {
    let mut words = heading.split_whitespace();
    let first = words.next()?;
    words.next()?;
    LEADING_ARTICLES
        .iter()
        .find(|article| first == **article)
        .copied()
}

/// The punctuation mark that ends a heading. A heading ending in a closing
/// quotation mark or italics markup ends inside a quoted title, and a period
/// that closes a name abbreviation or an initialism belongs to the name.
fn trailing_punctuation_of(heading: &str) -> Option<char> {
    let trimmed = heading.trim_end();
    let mark = trimmed.chars().last()?;
    if !TRAILING_PUNCTUATION.contains(&mark) {
        return None;
    }
    if mark == '.' {
        let last_word = trimmed.split_whitespace().last().unwrap_or("");
        if NAME_ABBREVIATIONS.contains(&last_word) || last_word.matches('.').count() > 1 {
            return None;
        }
    }
    Some(mark)
}

fn heading_markup_kinds(
    document: &ParsedArticleDocument,
    heading: &HeadingRecord,
) -> Vec<&'static str> {
    let within = |start: usize| start >= heading.start && start < heading.end;
    let mut kinds = Vec::new();
    if document
        .transclusions
        .iter()
        .any(|span| within(span.start) && !document.in_literal_region(span.start))
    {
        kinds.push("a template");
    }
    if document
        .references
        .iter()
        .any(|reference| within(reference.start) && !document.in_literal_region(reference.start))
    {
        kinds.push("a ref");
    }
    if document
        .parser_tags
        .iter()
        .any(|tag| tag.tag_name == "math" && within(tag.start))
    {
        kinds.push("math");
    }
    kinds
}

/// Lowercase words of the title without its namespace or a trailing
/// parenthetical qualifier such as "(band)".
fn title_words(document: &ParsedArticleDocument) -> Vec<String> {
    let title = document.title.as_str();
    let body = if document.namespace == Namespace::Main.as_str() {
        title
    } else {
        title.split_once(':').map(|(_, rest)| rest).unwrap_or(title)
    };
    let body = match body.trim_end().strip_suffix(')') {
        Some(stripped) => stripped
            .rsplit_once(" (")
            .map(|(name, _)| name)
            .unwrap_or(body),
        None => body,
    };
    words_of(body).into_iter().map(str::to_lowercase).collect()
}

/// Words split at anything but letters, digits and inner hyphens, so a
/// compound such as "left-accelerationism" stays one word.
fn words_of(value: &str) -> Vec<&str> {
    value
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '-'))
        .map(|word| word.trim_matches('-'))
        .filter(|word| !word.is_empty())
        .collect()
}

/// Whether the heading contains the title as whole words. A match followed by a
/// capitalized word or a number names something else that contains the title,
/// such as "Ethereum Foundation", "Miladycraft 1.4" or "Scatter API".
fn heading_repeats_title(heading: &str, title_words: &[String]) -> bool {
    let words = words_of(heading);
    let length = title_words.len();
    if length == 0 || words.len() < length {
        return false;
    }
    (0..=words.len() - length).any(|start| {
        let matches_title = words[start..start + length]
            .iter()
            .zip(title_words)
            .all(|(word, title_word)| word.to_lowercase() == *title_word);
        let extends_name = words.get(start + length).is_some_and(|next| {
            next.chars()
                .next()
                .is_some_and(|ch| ch.is_uppercase() || ch.is_ascii_digit())
        });
        matches_title && !extends_name
    })
}

pub(super) fn lint_consecutive_blank_lines(
    document: &ParsedArticleDocument,
    resources: &LoadedResources,
    matches: &mut Vec<IssueMatch>,
) {
    let Some(severity) = rule_severity(resources.adapter.lint.consecutive_blank_lines) else {
        return;
    };
    let lines = &document.lines;
    let mut index = 0usize;
    while index < lines.len() {
        if !is_blank(&lines[index]) {
            index += 1;
            continue;
        }
        let run_start = index;
        while index < lines.len() && is_blank(&lines[index]) {
            index += 1;
        }
        let run = &lines[run_start..index];
        // Trailing blank lines are trimmed by MediaWiki on save.
        let Some(next_line) = lines.get(index) else {
            break;
        };
        if run.len() < 2 || run.iter().any(|line| blank_line_is_opaque(document, line)) {
            continue;
        }
        let edit = TextEdit {
            start: run[1].start,
            end: next_line.start,
            replacement: String::new(),
        };
        let label = "Remove extra blank lines";
        let mut found = issue(
            "style.consecutive_blank_lines",
            severity,
            format!(
                "{} blank lines in a row; use one blank line between paragraphs.",
                run.len()
            ),
            document.span_for_range(run[1].start, run[run.len() - 1].end),
            format!("lines {}-{}", run[0].number, run[run.len() - 1].number),
            "Delete the extra blank lines so a single blank line separates the paragraphs."
                .to_string(),
        );
        found.issue.suggested_fixes = vec![safe_fix_for_edit(document, &edit, label)];
        found.safe_fixes = vec![SafeFixEdit {
            rule_id: "style.consecutive_blank_lines".to_string(),
            label: label.to_string(),
            line: Some(run[1].number),
            edit,
        }];
        matches.push(found);
    }
}

fn is_blank(line: &LineRecord) -> bool {
    line.text.trim().is_empty()
}

/// A blank line inside a comment, a literal tag, a template invocation or a ref
/// is not paragraph spacing.
fn blank_line_is_opaque(document: &ParsedArticleDocument, line: &LineRecord) -> bool {
    document.in_literal_region(line.start)
        || document.in_transclusion(line.start)
        || document
            .references
            .iter()
            .any(|reference| reference.start < line.start && line.start < reference.end)
}

pub(super) fn lint_lead_paragraph_limit(
    document: &ParsedArticleDocument,
    resources: &LoadedResources,
    matches: &mut Vec<IssueMatch>,
) {
    let lint = &resources.adapter.lint;
    let Some(severity) = rule_severity(lint.lead_paragraph_limit) else {
        return;
    };
    if document.namespace != Namespace::Main.as_str() || document.is_redirect {
        return;
    }
    let Some(lead) = document
        .sections
        .first()
        .filter(|section| section.heading.is_none())
    else {
        return;
    };
    let max = lint
        .max_lead_paragraphs
        .unwrap_or(DEFAULT_MAX_LEAD_PARAGRAPHS);

    let mut paragraphs = 0usize;
    let mut in_paragraph = false;
    let mut first_excess_line = None;
    for line in document
        .lines
        .iter()
        .filter(|line| line.start >= lead.body_start && line.start < lead.body_end)
    {
        if is_blank(line) {
            in_paragraph = false;
            continue;
        }
        if in_paragraph || !line_has_lead_prose(document, line) {
            continue;
        }
        in_paragraph = true;
        paragraphs += 1;
        if paragraphs == max + 1 {
            first_excess_line = Some(line);
        }
    }
    let Some(excess) = first_excess_line else {
        return;
    };
    matches.push(issue(
        "structure.lead_paragraph_limit",
        severity,
        format!("The lead has {paragraphs} paragraphs; this wiki allows at most {max}."),
        document.span_for_line(excess),
        format!("paragraphs={paragraphs} max={max}"),
        format!(
            "Move detail into body sections so the lead summarizes the article in {max} paragraphs or fewer."
        ),
    ));
}

/// Whether a lead line carries text a reader sees as a paragraph: something
/// left after removing templates, refs, comments and literal tags that is not
/// only category or file links, behavior switches such as `__NOTOC__`, or HTML
/// tags such as a wrapping `<div>`.
fn line_has_lead_prose(document: &ParsedArticleDocument, line: &LineRecord) -> bool {
    let visible = line
        .text
        .char_indices()
        .filter(|(index, _)| {
            let offset = line.start + index;
            !document.in_literal_region(offset)
                && !document.in_transclusion(offset)
                && !document
                    .references
                    .iter()
                    .any(|reference| offset >= reference.start && offset < reference.end)
        })
        .map(|(_, ch)| ch)
        .collect::<String>();
    !only_page_metadata(&visible)
}

fn only_page_metadata(value: &str) -> bool {
    let mut rest = value.trim_start();
    while !rest.is_empty() {
        if let Some(after_open) = rest.strip_prefix("[[") {
            let Some(close) = matching_link_close(after_open) else {
                return false;
            };
            let is_metadata = parse_wikilink(&after_open[..close]).is_some_and(|link| {
                !after_open.starts_with(':')
                    && (link.target_namespace == Namespace::Category.as_str()
                        || link.target_namespace == Namespace::File.as_str())
            });
            if !is_metadata {
                return false;
            }
            rest = after_open[close + 2..].trim_start();
            continue;
        }
        let skipped = behavior_switch_len(rest).max(html_tag_len(rest));
        if skipped == 0 {
            return false;
        }
        rest = rest[skipped..].trim_start();
    }
    true
}

/// Byte offset of the `]]` that closes a link whose `[[` was already consumed,
/// allowing links nested in a file caption.
fn matching_link_close(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    let mut depth = 0usize;
    let mut cursor = 0usize;
    while cursor + 1 < bytes.len() {
        match (bytes[cursor], bytes[cursor + 1]) {
            (b'[', b'[') => {
                depth += 1;
                cursor += 2;
            }
            (b']', b']') if depth == 0 => return Some(cursor),
            (b']', b']') => {
                depth -= 1;
                cursor += 2;
            }
            _ => cursor += 1,
        }
    }
    None
}

/// Length of a leading `__WORD__` behavior switch, or 0.
fn behavior_switch_len(value: &str) -> usize {
    let Some(body) = value.strip_prefix("__") else {
        return 0;
    };
    let name_len = body
        .bytes()
        .take_while(|byte| byte.is_ascii_uppercase())
        .count();
    if name_len == 0 || !body[name_len..].starts_with("__") {
        return 0;
    }
    name_len + 4
}

/// Length of a leading HTML start or end tag such as `<div class="x">` or
/// `</div>`, or 0. Quoted attribute values may contain `>`.
fn html_tag_len(value: &str) -> usize {
    let bytes = value.as_bytes();
    let name_start = match bytes {
        [b'<', b'/', ..] => 2,
        [b'<', ..] => 1,
        _ => return 0,
    };
    if !bytes.get(name_start).is_some_and(u8::is_ascii_alphabetic) {
        return 0;
    }
    let mut quote = None;
    for (index, byte) in bytes.iter().enumerate().skip(name_start) {
        match (quote, *byte) {
            (Some(open), current) if current == open => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(*byte),
            (None, b'>') => return index + 1,
            (None, _) => {}
        }
    }
    0
}
