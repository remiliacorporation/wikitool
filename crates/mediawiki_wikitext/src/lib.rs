#![forbid(unsafe_code)]

//! Deterministic structural parsing and explicit rewriting for MediaWiki source wikitext.
//!
//! The crate deliberately does not fetch pages, read corpus layouts, expand templates, or choose
//! target-wiki semantics. A caller supplies revision-bound UTF-8 bytes and a small parsing profile;
//! this crate verifies that neutral envelope, identifies the syntax needed by an archival
//! projector, and applies only caller-authorized, non-overlapping replacements.

mod parser;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SOURCE_PROFILE_SCHEMA: &str = "mediawiki.wikitext-source-profile.v1";
pub const SOURCE_DOCUMENT_SCHEMA: &str = "mediawiki.wikitext-source-document.v1";

/// Site syntax needed to classify links and recognize redirects without embedding a source site.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceProfile {
    pub schema: String,
    pub profile_id: String,
    pub source_key: String,
    pub max_wikitext_bytes: u64,
    pub max_nesting_depth: u32,
    pub file_namespace_aliases: BTreeSet<String>,
    pub category_namespace_aliases: BTreeSet<String>,
    pub redirect_magic_words: BTreeSet<String>,
}

impl SourceProfile {
    /// Constructs an English MediaWiki baseline. Callers should replace the aliases with the
    /// namespace and magic-word data reported by the source wiki when those data are available.
    pub fn mediawiki_defaults(
        profile_id: impl Into<String>,
        source_key: impl Into<String>,
        max_wikitext_bytes: u64,
    ) -> Self {
        Self {
            schema: SOURCE_PROFILE_SCHEMA.to_string(),
            profile_id: profile_id.into(),
            source_key: source_key.into(),
            max_wikitext_bytes,
            max_nesting_depth: 256,
            file_namespace_aliases: ["File", "Image"].map(str::to_string).into(),
            category_namespace_aliases: ["Category"].map(str::to_string).into(),
            redirect_magic_words: ["REDIRECT"].map(str::to_string).into(),
        }
    }
}

/// Revision and byte identity passed across an external corpus boundary.
///
/// This is intentionally a document envelope, not a corpus manifest. Private or site-specific
/// producers can translate their page records into this stable public contract without becoming a
/// Wikitool runtime dependency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceDocumentReceipt {
    pub schema: String,
    pub source_key: String,
    pub page_id: u64,
    pub namespace_id: i32,
    pub title: String,
    pub revision_id: u64,
    pub revision_timestamp: String,
    pub content_model: String,
    pub source_sha256: String,
    pub source_bytes: u64,
}

pub struct SourceDocumentInput<'wikitext, 'receipt, 'profile> {
    pub receipt: &'receipt SourceDocumentReceipt,
    pub source: &'profile SourceProfile,
    pub wikitext: &'wikitext str,
}

pub struct ProfiledParseInput<'wikitext, 'profile> {
    pub source: &'profile SourceProfile,
    pub wikitext: &'wikitext str,
}

/// Verifies a revision-bound source envelope before parsing its wikitext.
pub fn parse_source_document<'wikitext>(
    input: SourceDocumentInput<'wikitext, '_, '_>,
) -> Result<Document<'wikitext>, ParseError> {
    validate_source_profile(input.source)?;
    validate_source_document(input.receipt, input.source, input.wikitext)?;
    parser::parse(input.wikitext, input.source)
}

/// Parses wikitext under a validated source profile when document identity is managed separately.
pub fn parse_profiled<'wikitext>(
    input: ProfiledParseInput<'wikitext, '_>,
) -> Result<Document<'wikitext>, ParseError> {
    validate_source_profile(input.source)?;
    validate_input_size(input.wikitext, input.source.max_wikitext_bytes)?;
    parser::parse(input.wikitext, input.source)
}

pub fn validate_source_profile(source: &SourceProfile) -> Result<(), ParseError> {
    if source.schema != SOURCE_PROFILE_SCHEMA {
        return Err(ParseError::invalid_profile(format!(
            "source profile schema must be {SOURCE_PROFILE_SCHEMA}, got {}",
            source.schema
        )));
    }
    validate_identifier(&source.profile_id, "profile_id")?;
    validate_identifier(&source.source_key, "source_key")?;
    if source.max_wikitext_bytes == 0 {
        return Err(ParseError::invalid_profile(
            "max_wikitext_bytes must be greater than zero",
        ));
    }
    if source.max_nesting_depth == 0 {
        return Err(ParseError::invalid_profile(
            "max_nesting_depth must be greater than zero",
        ));
    }
    validate_aliases(&source.file_namespace_aliases, "file_namespace_aliases")?;
    validate_aliases(
        &source.category_namespace_aliases,
        "category_namespace_aliases",
    )?;
    for file_alias in &source.file_namespace_aliases {
        if source
            .category_namespace_aliases
            .iter()
            .any(|category_alias| normalized_words_equal(file_alias, category_alias))
        {
            return Err(ParseError::invalid_profile(format!(
                "namespace alias {file_alias:?} cannot classify both file and category links"
            )));
        }
    }
    if source.redirect_magic_words.is_empty() {
        return Err(ParseError::invalid_profile(
            "redirect_magic_words must not be empty",
        ));
    }
    for word in &source.redirect_magic_words {
        let normalized = word.trim();
        if normalized.is_empty()
            || normalized.starts_with('#')
            || normalized.contains('[')
            || normalized.contains(']')
        {
            return Err(ParseError::invalid_profile(format!(
                "redirect magic word {word:?} is not a bare magic word"
            )));
        }
    }
    Ok(())
}

fn validate_source_document(
    receipt: &SourceDocumentReceipt,
    source: &SourceProfile,
    wikitext: &str,
) -> Result<(), ParseError> {
    if receipt.schema != SOURCE_DOCUMENT_SCHEMA {
        return Err(ParseError::invalid_receipt(format!(
            "source document schema must be {SOURCE_DOCUMENT_SCHEMA}, got {}",
            receipt.schema
        )));
    }
    if receipt.source_key != source.source_key {
        return Err(ParseError::invalid_receipt(format!(
            "receipt source_key {:?} does not match profile source_key {:?}",
            receipt.source_key, source.source_key
        )));
    }
    if receipt.page_id == 0 {
        return Err(ParseError::invalid_receipt(
            "page_id must be greater than zero",
        ));
    }
    if receipt.revision_id == 0 {
        return Err(ParseError::invalid_receipt(
            "revision_id must be greater than zero",
        ));
    }
    if receipt.title.trim().is_empty() {
        return Err(ParseError::invalid_receipt("title must not be empty"));
    }
    if receipt.revision_timestamp.trim().is_empty() {
        return Err(ParseError::invalid_receipt(
            "revision_timestamp must not be empty",
        ));
    }
    if receipt.content_model != "wikitext" {
        return Err(ParseError::invalid_receipt(format!(
            "content_model must be wikitext, got {:?}",
            receipt.content_model
        )));
    }
    validate_input_size(wikitext, source.max_wikitext_bytes)?;
    let actual_bytes = u64::try_from(wikitext.len()).map_err(|_| {
        ParseError::invalid_receipt("wikitext byte length cannot be represented as u64")
    })?;
    if receipt.source_bytes != actual_bytes {
        return Err(ParseError::invalid_receipt(format!(
            "source_bytes is {}, but the supplied wikitext has {actual_bytes} bytes",
            receipt.source_bytes
        )));
    }
    if !is_lower_hex_digest(&receipt.source_sha256) {
        return Err(ParseError::invalid_receipt(
            "source_sha256 must be a lowercase 64-character hexadecimal digest",
        ));
    }
    let actual_sha256 = format!("{:x}", Sha256::digest(wikitext.as_bytes()));
    if receipt.source_sha256 != actual_sha256 {
        return Err(ParseError::invalid_receipt(format!(
            "source_sha256 does not match the supplied wikitext (expected {actual_sha256})"
        )));
    }
    Ok(())
}

fn validate_input_size(wikitext: &str, max_bytes: u64) -> Result<(), ParseError> {
    let actual_bytes = wikitext.len() as u64;
    if actual_bytes > max_bytes {
        return Err(ParseError::new(
            ParseErrorKind::InputTooLarge {
                actual_bytes,
                max_bytes,
            },
            0,
        ));
    }
    Ok(())
}

fn normalized_words_equal(left: &str, right: &str) -> bool {
    normalize_mediawiki_word(left) == normalize_mediawiki_word(right)
}

fn normalize_mediawiki_word(value: &str) -> String {
    let mut output = String::new();
    let mut pending_space = false;
    for character in value.trim().chars() {
        if character == '_' || character.is_whitespace() {
            pending_space = !output.is_empty();
            continue;
        }
        if pending_space {
            output.push(' ');
            pending_space = false;
        }
        output.extend(character.to_lowercase());
    }
    output
}

fn validate_identifier(value: &str, label: &str) -> Result<(), ParseError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ParseError::invalid_profile(format!(
            "{label} must contain only ASCII letters, digits, '.', '-' or '_'"
        )));
    }
    Ok(())
}

fn validate_aliases(aliases: &BTreeSet<String>, label: &str) -> Result<(), ParseError> {
    if aliases.is_empty() {
        return Err(ParseError::invalid_profile(format!(
            "{label} must not be empty"
        )));
    }
    for alias in aliases {
        let normalized = alias.trim();
        if normalized.is_empty() || normalized.contains(':') || normalized.starts_with('#') {
            return Err(ParseError::invalid_profile(format!(
                "namespace alias {alias:?} in {label} is invalid"
            )));
        }
    }
    Ok(())
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

impl NodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn start(self) -> usize {
        self.start
    }

    pub fn end(self) -> usize {
        self.end
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document<'a> {
    source: &'a str,
    nodes: Vec<Node>,
    redirect: Option<Redirect>,
    tag_regions: Vec<TagRegion>,
}

impl<'a> Document<'a> {
    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)
    }

    pub fn text(&self, span: Span) -> &'a str {
        &self.source[span.start..span.end]
    }

    pub fn redirect(&self) -> Option<&Redirect> {
        self.redirect.as_ref()
    }

    /// The `includeonly`, `noinclude`, `onlyinclude` and `gallery` regions, in source order.
    /// They are not nodes: their contents are parsed like the text around them.
    pub fn tag_regions(&self) -> &[TagRegion] {
        &self.tag_regions
    }

    /// The regions that contain a byte offset, outermost first.
    pub fn tag_regions_at(&self, offset: usize) -> impl Iterator<Item = &TagRegion> {
        self.tag_regions
            .iter()
            .filter(move |region| region.span.start <= offset && offset < region.span.end)
    }

    /// Resolves a plan into the exact source byte ranges it replaces, in source order. Every
    /// other byte of the source is kept. Ancestor/descendant replacements, an insertion inside a
    /// node or another edit, and any other overlap are rejected so the caller must make the
    /// intended rewrite authority explicit.
    pub fn resolve(&self, plan: &RewritePlan) -> Result<Vec<ResolvedEdit>, RewriteError> {
        let mut edits: Vec<PendingEdit> = Vec::with_capacity(plan.len());
        for (id, replacement) in &plan.replacements {
            let node = self.node(*id).ok_or(RewriteError::UnknownNode(*id))?;
            edits.push(PendingEdit {
                start: node.span.start,
                end: node.span.end,
                sequence: 0,
                replacement: replacement.clone(),
                origin: EditOrigin::Replace(*id),
                shrink_floor: None,
            });
        }
        self.resolve_line_removals(plan, &mut edits)?;
        for (sequence, (offset, text)) in plan.insertions.iter().enumerate() {
            let offset = *offset;
            if offset > self.source.len() || !self.source.is_char_boundary(offset) {
                return Err(RewriteError::InvalidInsertion { offset });
            }
            if let Some(node) = self
                .nodes
                .iter()
                .find(|node| node.span.start < offset && offset < node.span.end)
            {
                return Err(RewriteError::InsertionInsideNode {
                    offset,
                    node: node.id,
                });
            }
            edits.push(PendingEdit {
                start: offset,
                end: offset,
                sequence: sequence + 1,
                replacement: text.clone(),
                origin: EditOrigin::Insert(sequence),
                shrink_floor: None,
            });
        }
        edits.sort_by_key(|edit| (edit.start, edit.end, edit.sequence));

        for index in 1..edits.len() {
            let previous_end = edits[index - 1].end;
            let current = &mut edits[index];
            if current.start >= previous_end {
                continue;
            }
            if let Some(floor) = current.shrink_floor
                && previous_end <= floor
            {
                // A removed last line gives back the line break it borrowed from the line
                // before it when that line is removed too.
                current.start = previous_end;
                continue;
            }
            return Err(overlap_error(&edits[index - 1], &edits[index]));
        }

        Ok(edits
            .into_iter()
            .map(|edit| ResolvedEdit {
                span: Span {
                    start: edit.start,
                    end: edit.end,
                },
                replacement: edit.replacement,
                origin: edit.origin,
            })
            .collect())
    }

    /// Applies a plan: [`Document::resolve`], then each replacement in source order.
    pub fn rewrite(&self, plan: &RewritePlan) -> Result<String, RewriteError> {
        let edits = self.resolve(plan)?;
        let replaced_bytes = edits.iter().map(|edit| edit.span.len()).sum::<usize>();
        let replacement_bytes = edits
            .iter()
            .map(|edit| edit.replacement.len())
            .sum::<usize>();
        let mut output = String::with_capacity(
            self.source
                .len()
                .saturating_sub(replaced_bytes)
                .saturating_add(replacement_bytes),
        );
        let mut cursor = 0;
        for edit in &edits {
            output.push_str(&self.source[cursor..edit.span.start]);
            output.push_str(&edit.replacement);
            cursor = edit.span.end;
        }
        output.push_str(&self.source[cursor..]);
        Ok(output)
    }

    /// Line removals, grouped by the line their node starts on. When nothing but spaces, tabs
    /// and other removed nodes is left on a line, the whole line goes with its line break (the
    /// last line gives up the break before it instead). Otherwise each node goes alone, with
    /// the spaces after it when it starts the line, so the line does not begin with a space.
    fn resolve_line_removals(
        &self,
        plan: &RewritePlan,
        edits: &mut Vec<PendingEdit>,
    ) -> Result<(), RewriteError> {
        let bytes = self.source.as_bytes();
        let mut by_line: BTreeMap<usize, Vec<&Node>> = BTreeMap::new();
        for id in &plan.line_removals {
            let node = self.node(*id).ok_or(RewriteError::UnknownNode(*id))?;
            let line_start = bytes[..node.span.start]
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |index| index + 1);
            by_line.entry(line_start).or_default().push(node);
        }
        for (line_start, mut nodes) in by_line {
            nodes.sort_by_key(|node| (node.span.start, node.span.end));
            for pair in nodes.windows(2) {
                if pair[1].span.start < pair[0].span.end {
                    return Err(RewriteError::OverlappingNodes {
                        first: pair[0].id,
                        second: pair[1].id,
                    });
                }
            }
            let last_end = nodes.last().map_or(line_start, |node| node.span.end);
            let line_end = bytes[last_end..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |index| last_end + index);
            let content_end =
                if line_end > last_end && line_end < bytes.len() && bytes[line_end - 1] == b'\r' {
                    line_end - 1
                } else {
                    line_end
                };
            let mut gaps_blank = true;
            let mut cursor = line_start;
            for node in &nodes {
                gaps_blank &= is_horizontal_space(&bytes[cursor..node.span.start]);
                cursor = node.span.end;
            }
            gaps_blank &= is_horizontal_space(&bytes[cursor..content_end]);
            let ids = nodes.iter().map(|node| node.id).collect::<Vec<_>>();
            if gaps_blank {
                let (start, end, shrink_floor) = if line_end < bytes.len() {
                    (line_start, line_end + 1, None)
                } else if line_start > 0 {
                    let break_start = if line_start >= 2 && bytes[line_start - 2] == b'\r' {
                        line_start - 2
                    } else {
                        line_start - 1
                    };
                    (break_start, bytes.len(), Some(line_start))
                } else {
                    (0, bytes.len(), None)
                };
                edits.push(PendingEdit {
                    start,
                    end,
                    sequence: 0,
                    replacement: String::new(),
                    origin: EditOrigin::RemoveLine(ids),
                    shrink_floor,
                });
                continue;
            }
            let mut covered = line_start;
            for node in nodes {
                let mut end = node.span.end;
                if node.span.start == covered {
                    while end < content_end && matches!(bytes[end], b' ' | b'\t') {
                        end += 1;
                    }
                }
                edits.push(PendingEdit {
                    start: node.span.start,
                    end,
                    sequence: 0,
                    replacement: String::new(),
                    origin: EditOrigin::RemoveLine(vec![node.id]),
                    shrink_floor: None,
                });
                covered = end;
            }
        }
        Ok(())
    }
}

fn is_horizontal_space(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| matches!(*byte, b' ' | b'\t'))
}

struct PendingEdit {
    start: usize,
    end: usize,
    sequence: usize,
    replacement: String,
    origin: EditOrigin,
    /// The start a last-line removal may move forward to, past the line break it borrowed.
    shrink_floor: Option<usize>,
}

fn overlap_error(first: &PendingEdit, second: &PendingEdit) -> RewriteError {
    match (first.origin.first_node(), second.origin.first_node()) {
        (Some(first), Some(second)) => RewriteError::OverlappingNodes { first, second },
        _ => RewriteError::InsertionInsideEdit {
            offset: if matches!(first.origin, EditOrigin::Insert(_)) {
                first.start
            } else {
                second.start
            },
        },
    }
}

/// One resolved edit: the exact source bytes it replaces and what replaces them. An insertion
/// has an empty span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEdit {
    span: Span,
    replacement: String,
    origin: EditOrigin,
}

impl ResolvedEdit {
    pub fn span(&self) -> Span {
        self.span
    }

    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    pub fn origin(&self) -> &EditOrigin {
        &self.origin
    }
}

/// The plan entry an edit comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOrigin {
    Replace(NodeId),
    /// The nodes whose line removal this edit is; one edit removes a whole line's nodes.
    RemoveLine(Vec<NodeId>),
    /// The index of the insertion, in the order the plan received them.
    Insert(usize),
}

impl EditOrigin {
    fn first_node(&self) -> Option<NodeId> {
        match self {
            Self::Replace(node) => Some(*node),
            Self::RemoveLine(nodes) => nodes.first().copied(),
            Self::Insert(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    id: NodeId,
    parent: Option<NodeId>,
    depth: usize,
    span: Span,
    kind: NodeKind,
}

impl Node {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Template(Template),
    Parameter(Parameter),
    InternalLink(InternalLink),
    Protected(ProtectedRegion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub name: Span,
    pub arguments: Vec<TemplateArgument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateArgument {
    /// The argument bytes excluding the leading `|`, including source whitespace.
    pub span: Span,
    /// The trimmed name before the first top-level `=`, or `None` for a positional argument.
    pub name: Option<Span>,
    /// The trimmed positional value or named value after the first top-level `=`.
    pub value: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: Span,
    /// Everything after the first top-level `|`; further pipes remain part of the default.
    pub default: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalLink {
    /// The complete trimmed target, including any `#fragment` suffix.
    pub target: Span,
    /// The target title before `#`, or an empty span for a fragment-only link.
    pub title: Span,
    /// The fragment text after the first `#`, excluding the marker.
    pub fragment: Option<Span>,
    /// Link components after the target. For file links these retain each option/caption segment.
    pub components: Vec<Span>,
    pub kind: LinkKind,
    pub leading_colon: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Main,
    File,
    Category,
    Other,
    Fragment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedRegion {
    pub kind: ProtectedKind,
    /// The bytes between the opening and closing markers, which are never parsed. Empty for a
    /// self-closing tag.
    pub inner: Span,
}

/// A tag whose region the parser reports without making it a node: the transclusion controls,
/// whose contents MediaWiki shows or hides depending on where the page is used, and galleries,
/// whose contents are one file per line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkupTag {
    IncludeOnly,
    NoInclude,
    OnlyInclude,
    Gallery,
}

impl MarkupTag {
    pub fn name(self) -> &'static str {
        match self {
            Self::IncludeOnly => "includeonly",
            Self::NoInclude => "noinclude",
            Self::OnlyInclude => "onlyinclude",
            Self::Gallery => "gallery",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagRegion {
    pub tag: MarkupTag,
    /// The opening tag through the closing tag, or to the end of the source when unclosed.
    pub span: Span,
    pub inner: Span,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedKind {
    Comment,
    Nowiki,
    Pre,
    Source,
    SyntaxHighlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Redirect {
    pub keyword: Span,
    pub link: NodeId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RewritePlan {
    replacements: BTreeMap<NodeId, String>,
    line_removals: BTreeSet<NodeId>,
    insertions: Vec<(usize, String)>,
}

impl RewritePlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace(
        &mut self,
        node: NodeId,
        replacement: impl Into<String>,
    ) -> Result<(), RewritePlanError> {
        self.ensure_unplanned(node)?;
        self.replacements.insert(node, replacement.into());
        Ok(())
    }

    pub fn remove(&mut self, node: NodeId) -> Result<(), RewritePlanError> {
        self.replace(node, String::new())
    }

    /// Removes a node together with its line when nothing else but spaces, tabs and other
    /// line-removed nodes is on it; see [`Document::resolve`].
    pub fn remove_line(&mut self, node: NodeId) -> Result<(), RewritePlanError> {
        self.ensure_unplanned(node)?;
        self.line_removals.insert(node);
        Ok(())
    }

    /// Inserts text at a byte offset that no node spans across. Several insertions at one
    /// offset keep the order they were added in, and precede a replacement starting there.
    pub fn insert(&mut self, offset: usize, text: impl Into<String>) {
        self.insertions.push((offset, text.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.replacements.len() + self.line_removals.len() + self.insertions.len()
    }

    fn ensure_unplanned(&self, node: NodeId) -> Result<(), RewritePlanError> {
        if self.replacements.contains_key(&node) || self.line_removals.contains(&node) {
            return Err(RewritePlanError::DuplicateNode(node));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewritePlanError {
    DuplicateNode(NodeId),
}

impl Display for RewritePlanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode(node) => {
                write!(formatter, "node {} already has a replacement", node.index())
            }
        }
    }
}

impl Error for RewritePlanError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteError {
    UnknownNode(NodeId),
    OverlappingNodes {
        first: NodeId,
        second: NodeId,
    },
    /// The offset is past the end of the source or inside a UTF-8 character.
    InvalidInsertion {
        offset: usize,
    },
    InsertionInsideNode {
        offset: usize,
        node: NodeId,
    },
    /// The insertion falls inside the bytes another edit replaces.
    InsertionInsideEdit {
        offset: usize,
    },
}

impl Display for RewriteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(formatter, "unknown node {}", node.index()),
            Self::OverlappingNodes { first, second } => write!(
                formatter,
                "replacement nodes {} and {} overlap",
                first.index(),
                second.index()
            ),
            Self::InvalidInsertion { offset } => write!(
                formatter,
                "insertion offset {offset} is not a character boundary of the source"
            ),
            Self::InsertionInsideNode { offset, node } => write!(
                formatter,
                "insertion offset {offset} is inside node {}",
                node.index()
            ),
            Self::InsertionInsideEdit { offset } => write!(
                formatter,
                "insertion offset {offset} is inside the bytes another edit replaces"
            ),
        }
    }
}

impl Error for RewriteError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    kind: ParseErrorKind,
    byte_offset: usize,
}

impl ParseError {
    fn new(kind: ParseErrorKind, byte_offset: usize) -> Self {
        Self { kind, byte_offset }
    }

    fn invalid_profile(message: impl Into<String>) -> Self {
        Self::new(ParseErrorKind::InvalidProfile(message.into()), 0)
    }

    fn invalid_receipt(message: impl Into<String>) -> Self {
        Self::new(ParseErrorKind::InvalidReceipt(message.into()), 0)
    }

    pub fn kind(&self) -> &ParseErrorKind {
        &self.kind
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.kind, self.byte_offset)
    }
}

impl Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    InvalidProfile(String),
    InvalidReceipt(String),
    InputTooLarge { actual_bytes: u64, max_bytes: u64 },
    NestingTooDeep { depth: usize, max_depth: u32 },
    UnclosedTemplate,
    UnclosedParameter,
    UnclosedInternalLink,
    UnclosedProtected(ProtectedKind),
    MalformedProtectedTag(ProtectedKind),
}

impl Display for ParseErrorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(message) => write!(formatter, "invalid source profile: {message}"),
            Self::InvalidReceipt(message) => {
                write!(formatter, "invalid source document receipt: {message}")
            }
            Self::InputTooLarge {
                actual_bytes,
                max_bytes,
            } => write!(
                formatter,
                "wikitext is {actual_bytes} bytes, exceeding the {max_bytes}-byte profile limit"
            ),
            Self::NestingTooDeep { depth, max_depth } => write!(
                formatter,
                "wikitext nesting depth {depth} exceeds the profile limit {max_depth}"
            ),
            Self::UnclosedTemplate => formatter.write_str("unclosed template invocation"),
            Self::UnclosedParameter => formatter.write_str("unclosed template parameter"),
            Self::UnclosedInternalLink => formatter.write_str("unclosed internal link"),
            Self::UnclosedProtected(kind) => write!(formatter, "unclosed {kind:?} region"),
            Self::MalformedProtectedTag(kind) => {
                write!(formatter, "malformed opening tag for {kind:?} region")
            }
        }
    }
}

fn trim_span(source: &str, mut start: usize, mut end: usize) -> Span {
    while start < end {
        let Some(character) = source[start..end].chars().next() else {
            break;
        };
        if !character.is_whitespace() {
            break;
        }
        start += character.len_utf8();
    }
    while start < end {
        let Some(character) = source[start..end].chars().next_back() else {
            break;
        };
        if !character.is_whitespace() {
            break;
        }
        end -= character.len_utf8();
    }
    Span { start, end }
}
