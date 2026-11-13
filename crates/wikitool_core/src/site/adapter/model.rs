use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdapterSourceDocument {
    pub relative_path: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CitationTemplateRule {
    pub family: String,
    pub template_title: String,
}

/// A deterministic URL matcher that asks for source review. Matching is not a
/// reliability verdict: the agent-owned review procedure decides whether the
/// source is appropriate for the particular claim and subject.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceReviewRule {
    pub label: String,
    pub host: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InfoboxPreference {
    pub subject_type: String,
    pub template_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthoringRules {
    pub require_short_description: bool,
    pub short_description_forms: Vec<String>,
    pub require_article_quality_banner: bool,
    pub article_quality_template: Option<String>,
    pub article_quality_default_state: Option<String>,
    pub required_appendix_sections: Vec<String>,
    pub references_template: Option<String>,
    pub prefer_sentence_case_headings: bool,
    pub prefer_wikitext_only: bool,
    pub forbid_markdown: bool,
    pub require_straight_quotes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CitationRules {
    pub preferred_templates: Vec<CitationTemplateRule>,
    pub use_named_references: bool,
    pub leave_archive_fields_blank: bool,
    #[serde(default)]
    pub source_review_rules: Vec<SourceReviewRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplateRules {
    pub infobox_preferences: Vec<InfoboxPreference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CategoryRules {
    pub preferred_categories: Vec<String>,
}

/// Deterministic, locally decidable lint configuration. Reader value, source
/// fidelity, due weight, BLP judgment, and prose quality deliberately do not
/// belong here; those are agent-skill and human-editor responsibilities.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LintRuleLevel {
    #[default]
    Ignore,
    Warning,
    Error,
}

impl LintRuleLevel {
    pub fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignore)
    }
}

/// The level of a rule that cannot tell a style fault from a name it matches,
/// such as a heading that begins with "The" because a band is called that.
/// Such a rule can only warn; an adapter cannot make it block acceptance.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LintAdvisoryLevel {
    #[default]
    Ignore,
    Warning,
}

impl LintAdvisoryLevel {
    pub fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignore)
    }
}

/// Lead paragraphs allowed when `lead_paragraph_limit` is enabled without
/// `max_lead_paragraphs`.
pub const DEFAULT_MAX_LEAD_PARAGRAPHS: usize = 4;

/// Style rule levels default to off and are omitted from the serialized policy
/// while off, so adding a rule leaves the publication policy hash of an adapter
/// that does not enable it unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LintRules {
    pub forbid_curly_quotes: bool,
    pub forbid_placeholder_fragments: Vec<String>,
    /// Proper nouns that may remain capitalized mid-heading. Local page titles
    /// are folded in automatically.
    #[serde(default)]
    pub proper_nouns: Vec<String>,
    /// Whether an unresolved `{{Citation needed}}` marker is acceptable on the
    /// target wiki. MediaWiki itself does not impose this editorial policy.
    #[serde(default)]
    pub citation_needed: LintRuleLevel,
    /// `style.heading_leading_article`: a heading begins with "The", "A" or "An".
    #[serde(default, skip_serializing_if = "LintAdvisoryLevel::is_ignore")]
    pub heading_leading_article: LintAdvisoryLevel,
    /// `style.heading_trailing_punctuation`: a heading ends in punctuation.
    #[serde(default, skip_serializing_if = "LintAdvisoryLevel::is_ignore")]
    pub heading_trailing_punctuation: LintAdvisoryLevel,
    /// `style.heading_markup`: a heading contains a template, a ref or math.
    #[serde(default, skip_serializing_if = "LintRuleLevel::is_ignore")]
    pub heading_markup: LintRuleLevel,
    /// `style.heading_repeats_title`: a heading repeats the article title.
    #[serde(default, skip_serializing_if = "LintAdvisoryLevel::is_ignore")]
    pub heading_repeats_title: LintAdvisoryLevel,
    /// `style.consecutive_blank_lines`: two or more blank lines in a row.
    #[serde(default, skip_serializing_if = "LintRuleLevel::is_ignore")]
    pub consecutive_blank_lines: LintRuleLevel,
    /// `structure.lead_paragraph_limit`: the lead has more paragraphs than
    /// `max_lead_paragraphs`.
    #[serde(default, skip_serializing_if = "LintRuleLevel::is_ignore")]
    pub lead_paragraph_limit: LintRuleLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lead_paragraphs: Option<usize>,
}

/// The ontology a wiki types its pages and infobox fields with. The adapter
/// names it so `wikitool ontology` needs no flags for the adapter's wiki.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OntologyRules {
    /// The wiki's product name in the ontology's `product-uses.tsv`.
    pub product: String,
    /// Where the ontology is published.
    pub source: String,
    /// The prefix of the wiki's own terms, such as `wiki`.
    pub local_prefix: String,
    /// The namespace IRI of the wiki's own terms.
    pub local_namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SiteAdapter {
    pub schema_version: String,
    pub adapter_id: String,
    pub docs_profile: String,
    /// The adapter TOML and any project-owned supplemental guidance named by it.
    /// Wikitool hashes and exposes these documents but never interprets prose as
    /// machine policy.
    pub source_documents: Vec<AdapterSourceDocument>,
    pub authoring: AuthoringRules,
    pub citations: CitationRules,
    pub templates: TemplateRules,
    pub categories: CategoryRules,
    pub lint: LintRules,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension_contracts: Vec<ExtensionContractRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ontology: Option<OntologyRules>,
    pub resolved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionContractRule {
    /// Mechanism kind: `tag`, `parser_function`, `template`, or `module`.
    pub kind: String,
    /// Invocable name: tag name, parser-function name (no `#`), template name,
    /// or module title tail.
    pub name: String,
    /// Providing extension (or `core` / `local`).
    pub provider: String,
    /// Invocation syntax: `paired`, `self_closing`, `parser_function`,
    /// `template`, or `module_invoke`.
    pub syntax: String,
    #[serde(default)]
    pub body_required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub example: String,
}

impl SiteAdapter {
    pub fn adapter_template_titles(&self) -> Vec<String> {
        let mut titles = BTreeSet::new();
        if let Some(value) = self.authoring.article_quality_template.as_deref() {
            titles.insert(value.to_string());
        }
        if let Some(value) = self.authoring.references_template.as_deref() {
            titles.insert(value.to_string());
        }
        for rule in &self.citations.preferred_templates {
            titles.insert(rule.template_title.clone());
        }
        for preference in &self.templates.infobox_preferences {
            titles.insert(preference.template_title.clone());
        }
        titles.into_iter().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplateCatalogSummary {
    pub site_adapter_id: String,
    pub template_count: usize,
    pub templatedata_count: usize,
    pub redirect_alias_count: usize,
    pub usage_index_ready: bool,
    pub adapter_template_titles: Vec<String>,
    pub refreshed_at: String,
}
