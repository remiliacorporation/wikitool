//! Lint one file wherever it lives. Reading is not a project write, so the path
//! may sit outside the project's content directories; the page title comes from
//! `--title`, the metadata header that `export --format wikitext` writes, or the
//! file's place under `wiki_content/` or `templates/`, in that order.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use super::document::parse_article_document;
use super::{ArticleLintReport, build_report, collect_issue_matches, load_resources};
use crate::content_store::parsing::normalize_query_title;
use crate::filesystem::relative_path_to_title;
use crate::runtime::ResolvedPaths;
use crate::support::normalize_path;

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
    #[serde(flatten)]
    pub report: ArticleLintReport,
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
    let absolute = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        paths.project_root.join(file_path)
    };
    let content = fs::read_to_string(&absolute)
        .with_context(|| format!("failed to read {}", normalize_path(&absolute)))?;
    let project_relative = absolute
        .strip_prefix(&paths.project_root)
        .ok()
        .map(normalize_path);
    let display_path = project_relative
        .clone()
        .unwrap_or_else(|| normalize_path(&absolute));

    let header = parse_export_metadata_header(&content);
    let body = match &header {
        Some((_, body_offset)) => &content[*body_offset..],
        None => content.as_str(),
    };

    let title = match title_override.map(normalize_query_title) {
        Some(title) if title.is_empty() => bail!("article title override must not be empty"),
        Some(title) => title,
        None => match (&header, &project_relative) {
            (Some((header, _)), _) => normalize_query_title(&header.title),
            (None, Some(relative)) => {
                relative_path_to_title(paths, relative).with_context(|| {
                    format!("cannot derive a page title for {display_path}; pass --title TITLE")
                })?
            }
            (None, None) => bail!(
                "cannot derive a page title for {display_path}: it has no export metadata header and is outside the project; pass --title TITLE"
            ),
        },
    };

    let resources = load_resources(paths)?;
    let document = parse_article_document(display_path, title, body.to_string());
    let matches = collect_issue_matches(paths, &document, &resources)?;
    let mut report = build_report(&document, &resources, matches);
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
