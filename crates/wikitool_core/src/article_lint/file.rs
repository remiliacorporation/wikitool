//! Lint one file wherever it lives. Reading is not a project write, so the path
//! may sit outside the project's content directories.
//!
//! A file under a project-owned root (`wiki_content/`, `templates/` or
//! `.wikitool/`, the paths accept, promote and push read) is page text as it
//! stands: they record and publish every byte. It is linted whole under the title its path gives (or `--title`), and an
//! export metadata header in it is an error. Any other file may be an export:
//! its header supplies the title unless `--title` is given, and is set aside
//! rather than linted as page text.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use super::document::parse_article_document;
use super::{
    ArticleLintIssue, ArticleLintReport, ArticleLintResources, ArticleLintSeverity, TextSpan,
    build_report, collect_issue_matches, load_article_lint_resources,
};
use crate::content_store::parsing::normalize_query_title;
use crate::filesystem::relative_path_to_title;
use crate::runtime::ResolvedPaths;
use crate::support::{compute_sha256, normalize_path};

/// Rule raised when a file on a publication path starts with an export header.
const EXPORT_HEADER_ON_PUBLICATION_PATH_RULE: &str = "structure.export_header_on_publication_path";

/// The metadata header `export --format wikitext` writes above the page text.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExportMetadataHeader {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<String>,
    /// Lines the header occupies. Lint reads only the page text after it and
    /// reports issue lines counted from the top of the file.
    pub line_count: usize,
}

/// A lint report for one file, with the export header it carried, if any.
#[derive(Debug, Clone, Serialize)]
pub struct ArticleFileLintReport {
    /// `content_sha256` inside covers the page text that was linted: the text
    /// after the export header when `export_header` is present, else the file.
    #[serde(flatten)]
    pub report: ArticleLintReport,
    /// SHA-256 of the whole file's bytes, the value `article accept
    /// --content-sha256` compares. It differs from `content_sha256` only when an
    /// export header was set aside.
    pub file_sha256: String,
    /// Present when the header was set aside: the file is outside
    /// `wiki_content/`, `templates/` and `.wikitool/` and starts with an export
    /// metadata header. `article accept` reads only those roots, so it never
    /// takes a file whose header was set aside.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_header: Option<ExportMetadataHeader>,
}

/// Lint `file_path` without requiring it to be under the project's content
/// directories. A relative path is resolved against the project root.
pub fn lint_article_file(
    paths: &ResolvedPaths,
    file_path: &Path,
    title_override: Option<&str>,
) -> Result<ArticleFileLintReport> {
    let resources = load_article_lint_resources(paths)?;
    lint_article_file_with_resources(paths, file_path, title_override, &resources)
}

/// [`lint_article_file`] with resources loaded once for a batch.
pub fn lint_article_file_with_resources(
    paths: &ResolvedPaths,
    file_path: &Path,
    title_override: Option<&str>,
    resources: &ArticleLintResources,
) -> Result<ArticleFileLintReport> {
    let resources = &resources.inner;
    let absolute = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        paths.project_root.join(file_path)
    };
    let content = fs::read_to_string(&absolute)
        .with_context(|| format!("failed to read {}", normalize_path(&absolute)))?;
    let file_sha256 = compute_sha256(&content);

    // Compare canonical forms so an absolute or differently spelled path to a
    // mirrored page is still recognized as one.
    let canonical_file = fs::canonicalize(&absolute).ok();
    let canonical_root = fs::canonicalize(&paths.project_root).ok();
    let project_relative = match (&canonical_file, &canonical_root) {
        (Some(file), Some(root)) => file.strip_prefix(root).ok().map(normalize_path),
        _ => None,
    }
    .or_else(|| {
        absolute
            .strip_prefix(&paths.project_root)
            .ok()
            .map(normalize_path)
    });
    let display_path = project_relative
        .clone()
        .unwrap_or_else(|| normalize_path(&absolute));
    let is_under = |directory: &Path| match (&canonical_file, fs::canonicalize(directory)) {
        (Some(file), Ok(directory)) => file.starts_with(directory),
        _ => false,
    };
    let on_publication_path = is_under(&paths.wiki_content_dir)
        || is_under(&paths.templates_dir)
        || is_under(&paths.state_dir);

    let title_override = match title_override.map(normalize_query_title) {
        Some(title) if title.is_empty() => bail!("article title override must not be empty"),
        other => other,
    };
    let header = parse_export_metadata_header(&content);

    if on_publication_path {
        let title = match title_override {
            Some(title) => title,
            None => {
                let relative = project_relative.as_deref().unwrap_or(&display_path);
                relative_path_to_title(paths, relative)?
            }
        };
        let document = parse_article_document(display_path, title, content);
        let matches = collect_issue_matches(paths, &document, resources)?;
        let mut report = build_report(&document, resources, matches);
        if let Some((header, _)) = &header {
            report.issues.insert(
                0,
                ArticleLintIssue {
                    rule_id: EXPORT_HEADER_ON_PUBLICATION_PATH_RULE.to_string(),
                    severity: ArticleLintSeverity::Error,
                    message: format!(
                        "lines 1-{} are an export metadata header; this file is published as it stands, so promote or push would publish the header as page text",
                        header.line_count
                    ),
                    span: Some(TextSpan {
                        line: 1,
                        column: 1,
                        end_line: Some(header.line_count),
                        end_column: None,
                    }),
                    evidence: Some(format!("title: \"{}\"", header.title)),
                    suggested_remediation: Some(
                        "Delete the header block, from the first `---` line through the second."
                            .to_string(),
                    ),
                    suggested_fixes: Vec::new(),
                },
            );
            report.errors += 1;
            report.issue_count += 1;
        }
        return Ok(ArticleFileLintReport {
            report,
            file_sha256,
            export_header: None,
        });
    }

    let body = match &header {
        Some((_, body_offset)) => &content[*body_offset..],
        None => content.as_str(),
    };
    let title = match (title_override, &header, &project_relative) {
        (Some(title), _, _) => title,
        (None, Some((header, _)), _) => normalize_query_title(&header.title),
        (None, None, Some(relative)) => {
            relative_path_to_title(paths, relative).with_context(|| {
                format!("cannot derive a page title for {display_path}; pass --title TITLE")
            })?
        }
        (None, None, None) => bail!(
            "cannot derive a page title for {display_path}: it has no export metadata header and is outside the project; pass --title TITLE"
        ),
    };

    let document = parse_article_document(display_path, title, body.to_string());
    let matches = collect_issue_matches(paths, &document, resources)?;
    let mut report = build_report(&document, resources, matches);
    let export_header = header.map(|(header, _)| header);
    if let Some(header) = &export_header {
        for span in report
            .issues
            .iter_mut()
            .filter_map(|issue| issue.span.as_mut())
        {
            span.line += header.line_count;
            if let Some(end_line) = span.end_line.as_mut() {
                *end_line += header.line_count;
            }
        }
    }
    Ok(ArticleFileLintReport {
        report,
        file_sha256,
        export_header,
    })
}

/// Recognize the `---`-fenced block of `key: "value"` lines that export writes,
/// returning it and the byte offset where the page text begins. Anything else,
/// including a block without a `title`, is page text.
fn parse_export_metadata_header(content: &str) -> Option<(ExportMetadataHeader, usize)> {
    let mut lines = content.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return None;
    }
    let mut offset = first.len();
    let mut fields = BTreeMap::new();
    // The opening fence is line 1, so the first field is line 2.
    for (line_count, line) in (2..).zip(lines) {
        offset += line.len();
        let text = line.trim_end_matches(['\r', '\n']);
        if text == "---" {
            let title = fields.remove("title")?;
            return Some((
                ExportMetadataHeader {
                    title,
                    source_url: fields.remove("source_url"),
                    revision_id: fields.remove("revision_id"),
                    line_count,
                },
                offset,
            ));
        }
        let (key, value) = text.split_once(": ")?;
        if key.is_empty()
            || !key
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_')
        {
            return None;
        }
        let value = value.strip_prefix('"')?.strip_suffix('"')?;
        fields.insert(key.to_string(), value.replace("\\\"", "\""));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORTED: &str = "---\ntitle: \"Exocore \\\"draft\\\"\"\nsource_url: \"https://wiki.example/wiki/Exocore\"\nrevision_id: \"6082\"\n---\n{{SHORTDESC:Example}}\nBody\n";

    #[test]
    fn export_header_is_recognized_and_separated_from_page_text() {
        let (header, offset) = parse_export_metadata_header(EXPORTED).expect("header");
        assert_eq!(header.title, "Exocore \"draft\"");
        assert_eq!(
            header.source_url.as_deref(),
            Some("https://wiki.example/wiki/Exocore")
        );
        assert_eq!(header.revision_id.as_deref(), Some("6082"));
        assert_eq!(header.line_count, 5);
        assert_eq!(&EXPORTED[offset..], "{{SHORTDESC:Example}}\nBody\n");
    }

    #[test]
    fn crlf_headers_are_recognized() {
        let crlf = EXPORTED.replace('\n', "\r\n");
        let (header, offset) = parse_export_metadata_header(&crlf).expect("header");
        assert_eq!(header.line_count, 5);
        assert!(crlf[offset..].starts_with("{{SHORTDESC"));
    }

    #[test]
    fn page_text_that_only_resembles_a_header_is_not_one() {
        for content in [
            "{{Infobox}}\n---\ntitle: \"X\"\n---\n",
            "----\nA horizontal rule\n",
            "---\nnot a field\n---\n",
            "---\nsource_url: \"https://wiki.example\"\n---\nNo title\n",
            "---\ntitle: \"Unclosed\"\nBody\n",
        ] {
            assert!(
                parse_export_metadata_header(content).is_none(),
                "{content:?}"
            );
        }
    }
}
