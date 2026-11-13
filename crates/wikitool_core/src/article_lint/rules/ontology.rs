use crate::article_lint::model::{
    ArticleLintIssue, ArticleLintOntologyStatus, ArticleLintSeverity, ArticleLintSkippedCheck,
};
use crate::ontology::typing::{
    ArticleTyping, OntologyAvailability, PROPERTY_NOT_APPLICABLE, ProjectOntology,
};
use crate::site::LintRuleLevel;

use super::super::document::ParsedArticleDocument;
use super::super::resources::LoadedResources;
use super::IssueMatch;

/// `ontology.*`: the classes, infobox fields and values the article states
/// against the vendored ontology and the wiki's ontology modules. Runs only
/// when the adapter has `[ontology]` (at `warning` unless `ontology_typing` says otherwise) and the
/// project has the vendored export.
pub(super) fn lint_ontology_typing(
    document: &ParsedArticleDocument,
    resources: &LoadedResources,
    matches: &mut Vec<IssueMatch>,
) {
    let severity = match resources.adapter.ontology_typing_level() {
        LintRuleLevel::Ignore => return,
        LintRuleLevel::Warning => ArticleLintSeverity::Warning,
        LintRuleLevel::Error => ArticleLintSeverity::Error,
    };
    let Some(OntologyAvailability::Ready(project)) = resources.ontology.as_ref() else {
        return;
    };
    if document.is_redirect {
        return;
    }
    for finding in article_typing(document, resources, project).findings {
        // A property that does not apply comes from the display module, which
        // `ontology check` reports once; on an article it is only a note.
        let severity = if finding.code == PROPERTY_NOT_APPLICABLE {
            ArticleLintSeverity::Suggestion
        } else {
            severity
        };
        matches.push(IssueMatch {
            issue: ArticleLintIssue {
                rule_id: finding.code.to_string(),
                severity,
                message: finding.message,
                span: document.span_for_range(finding.start, finding.end),
                evidence: (!finding.evidence.is_empty()).then_some(finding.evidence),
                suggested_remediation: Some(finding.remediation),
                suggested_fixes: Vec::new(),
            },
            safe_fixes: Vec::new(),
        });
    }
}

/// Which typing checks ran on the article and which were skipped, or `None`
/// when the adapter does not enable them.
pub(in crate::article_lint) fn ontology_status(
    document: &ParsedArticleDocument,
    resources: &LoadedResources,
) -> Option<ArticleLintOntologyStatus> {
    if resources.adapter.ontology_typing_level().is_ignore() {
        return None;
    }
    let skipped = |check: &str, reason: String| ArticleLintSkippedCheck {
        check: check.to_string(),
        reason,
    };
    match resources.ontology.as_ref()? {
        OntologyAvailability::NotConfigured => None,
        OntologyAvailability::Unavailable { reason } => Some(ArticleLintOntologyStatus {
            ontology_commit: None,
            checks_run: Vec::new(),
            skipped: vec![skipped("all", reason.clone())],
        }),
        OntologyAvailability::Ready(project) => {
            let typing = if document.is_redirect {
                Default::default()
            } else {
                article_typing(document, resources, project)
            };
            Some(ArticleLintOntologyStatus {
                ontology_commit: project.commit().map(str::to_string),
                checks_run: typing
                    .checks_run
                    .iter()
                    .map(|check| check.to_string())
                    .collect(),
                skipped: typing
                    .skipped
                    .into_iter()
                    .map(|item| skipped(item.check, item.reason))
                    .collect(),
            })
        }
    }
}

/// The article's typing, computed once per lint run for both the findings
/// and the report's status.
fn article_typing(
    document: &ParsedArticleDocument,
    resources: &LoadedResources,
    project: &ProjectOntology,
) -> ArticleTyping {
    let mut cache = resources.ontology_typing.borrow_mut();
    if let Some((content, typing)) = cache.as_ref()
        && *content == document.content
    {
        return typing.clone();
    }
    let typing = project.check_article(&document.content);
    *cache = Some((document.content.clone(), typing.clone()));
    typing
}
