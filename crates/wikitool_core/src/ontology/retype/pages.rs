//! Where a title's page lives, and where the command may read and write.
//!
//! A title is an article when no namespace, built in or configured, claims
//! its prefix and the sync layer maps it to `wiki_content/Main/`. Its mirror
//! file is the one the sync layer stores it in, which `pull` gives a
//! case-safe name when two titles differ only in case. Its drafts are the
//! files under `.wikitool/drafts/`, at any depth, whose file name spells the
//! title the way the sync layer spells file names (`Old_Wojak.wiki`,
//! `Miya Notes.wiki`, or a case-safe name). Every path read or written is
//! checked to resolve inside its directory: drafts under `.wikitool/drafts/`,
//! mirror pages under `wiki_content/`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::load_config;
use crate::filesystem::{
    NamespaceMapper, case_safe_title_relative_path, resolve_existing_ancestor, validate_scoped_path,
};
use crate::runtime::ResolvedPaths;
use crate::support::normalize_path;

use super::{REFUSED_INVALID_TITLE, REFUSED_NOT_AN_ARTICLE, Refusal, normalize_title};

/// MediaWiki's built-in namespaces and their talk namespaces, in lower case.
const BUILT_IN_NAMESPACES: &[&str] = &[
    "media",
    "special",
    "talk",
    "user",
    "user talk",
    "project",
    "project talk",
    "file",
    "file talk",
    "image",
    "image talk",
    "mediawiki",
    "mediawiki talk",
    "template",
    "template talk",
    "help",
    "help talk",
    "category",
    "category talk",
    "module",
    "module talk",
];

/// Characters MediaWiki forbids in titles, and the path separator the sync
/// layer leaves in file names.
const TITLE_FORBIDDEN: &[char] = &['#', '<', '>', '[', ']', '|', '{', '}', '\\'];

const DRAFT_EXTENSIONS: &[&str] = &[".wiki", ".wikitext"];

pub(super) struct PageFiles<'p> {
    paths: &'p ResolvedPaths,
    mapper: NamespaceMapper,
    /// Lower-case namespace names that make a title not an article.
    namespaces: BTreeSet<String>,
    content_relative: String,
    content_main: String,
    drafts_relative: String,
    /// Drafts by normalized title.
    drafts: BTreeMap<String, Vec<String>>,
}

impl<'p> PageFiles<'p> {
    pub(super) fn load(paths: &'p ResolvedPaths) -> Result<Self> {
        let mapper = NamespaceMapper::load(paths)?;
        let config = load_config(&paths.config_path)?;
        let mut namespaces: BTreeSet<String> = BUILT_IN_NAMESPACES
            .iter()
            .map(|name| name.to_string())
            .collect();
        let custom = config
            .wiki
            .custom_namespaces
            .iter()
            .map(|namespace| namespace.name.clone())
            .chain(mapper.custom_folders());
        for name in custom {
            let name = namespace_key(&name);
            if !name.is_empty() {
                namespaces.insert(format!("{name} talk"));
                namespaces.insert(name);
            }
        }
        let content = relative_to_root(paths, &paths.wiki_content_dir);
        let drafts_dir = paths.state_dir.join("drafts");
        let mut files = Self {
            paths,
            mapper,
            namespaces,
            content_main: format!("{content}/Main/"),
            content_relative: content.clone(),
            drafts_relative: relative_to_root(paths, &drafts_dir),
            drafts: BTreeMap::new(),
        };
        files.index_drafts(&drafts_dir)?;
        Ok(files)
    }

    fn index_drafts(&mut self, directory: &Path) -> Result<()> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read {}", normalize_path(directory)));
            }
        };
        for entry in entries {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                self.index_drafts(&path)?;
                continue;
            }
            let relative = relative_to_root(self.paths, &path);
            if !DRAFT_EXTENSIONS
                .iter()
                .any(|extension| relative.ends_with(extension))
            {
                continue;
            }
            let title = self.draft_title(&relative);
            self.drafts.entry(title).or_default().push(relative);
        }
        Ok(())
    }

    /// The title a draft's file name spells.
    pub(super) fn draft_title(&self, relative: &str) -> String {
        normalize_title(&self.mapper.relative_path_to_title(self.paths, relative))
    }

    /// The title a mirror page's path stands for.
    pub(super) fn mirror_title(&self, relative: &str) -> String {
        normalize_title(&self.mapper.relative_path_to_title(self.paths, relative))
    }

    /// Whether a project-relative path is under `.wikitool/drafts/`.
    pub(super) fn is_draft_path(&self, relative: &str) -> bool {
        relative.starts_with(&format!("{}/", self.drafts_relative))
    }

    /// Whether a project-relative path is under `wiki_content/`.
    pub(super) fn is_mirror_path(&self, relative: &str) -> bool {
        relative.starts_with(&format!("{}/", self.content_relative))
    }

    /// Why a title is not an article the command may retype, if it is not.
    pub(super) fn article_refusal(&self, title: &str) -> Option<Refusal> {
        if title.contains(TITLE_FORBIDDEN) || title.chars().any(char::is_control) {
            return Some(Refusal::new(
                REFUSED_INVALID_TITLE,
                format!("{title:?} holds a character that cannot be in an article title"),
            ));
        }
        if let Some((prefix, _)) = title.split_once(':')
            && self.namespaces.contains(&namespace_key(prefix))
        {
            return Some(Refusal::new(
                REFUSED_NOT_AN_ARTICLE,
                format!(
                    "{title:?} is in the {} namespace, not the article namespace",
                    prefix.trim()
                ),
            ));
        }
        let mirror = self.mapper.title_to_relative_path(self.paths, title, false);
        let in_main = mirror
            .strip_prefix(&self.content_main)
            .is_some_and(|file| !file.contains('/') && !file.contains('\\'));
        if !in_main {
            return Some(Refusal::new(
                REFUSED_NOT_AN_ARTICLE,
                format!("{title:?} is not stored as an article: its mirror path is {mirror}"),
            ));
        }
        None
    }

    /// The drafts whose file name spells the title.
    pub(super) fn drafts_for(&self, title: &str) -> &[String] {
        self.drafts
            .get(title)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// The mirror file the sync layer stores a title in, when it exists: its
    /// case-safe name, else its plain name. Names are compared exactly, so on
    /// a case-insensitive file system a title never reads another title's
    /// file. Without one, the plain name.
    pub(super) fn mirror_file(&self, title: &str, redirect: bool) -> Result<(String, bool)> {
        let plain = self
            .mapper
            .title_to_relative_path(self.paths, title, redirect);
        let case_safe = case_safe_title_relative_path(&plain, title);
        for candidate in [&case_safe, &plain] {
            if self.exists_exactly(candidate)? {
                return Ok((candidate.clone(), true));
            }
        }
        Ok((plain, false))
    }

    /// The draft a title gets when it has none: the mirror file's name under
    /// `.wikitool/drafts/`.
    pub(super) fn default_draft(&self, mirror: &str) -> String {
        let file_name = mirror.rsplit('/').next().unwrap_or(mirror);
        format!("{}/{file_name}", self.drafts_relative)
    }

    /// Another file in a draft's directory whose name differs from it only in
    /// case, which a case-insensitive file system would write over.
    pub(super) fn case_twin(&self, relative: &str) -> Result<Option<String>> {
        let (directory, name) = split_file_name(relative);
        Ok(case_twin_in(&self.listing(directory)?, name).map(|other| {
            if directory.is_empty() {
                other.clone()
            } else {
                format!("{directory}/{other}")
            }
        }))
    }

    /// The absolute path of a draft, checked to resolve under
    /// `.wikitool/drafts/`.
    pub(super) fn checked_draft(&self, relative: &str) -> Result<PathBuf> {
        self.checked(relative, &self.paths.state_dir.join("drafts"))
    }

    /// The absolute path of a mirror page, checked to resolve under
    /// `wiki_content/`.
    pub(super) fn checked_mirror(&self, relative: &str) -> Result<PathBuf> {
        self.checked(relative, &self.paths.wiki_content_dir)
    }

    fn checked(&self, relative: &str, directory: &Path) -> Result<PathBuf> {
        let candidate = Path::new(relative);
        let plain = !relative.contains('\\')
            && candidate
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !plain {
            anyhow::bail!("refusing the path {relative:?}: it is not a plain relative path");
        }
        let absolute = self.paths.project_root.join(candidate);
        validate_scoped_path(self.paths, &absolute)?;
        let resolved = resolve_existing_ancestor(&absolute)?;
        let root = resolve_existing_ancestor(directory)?;
        if !resolved.starts_with(&root) {
            anyhow::bail!(
                "refusing the path {relative:?}: it resolves to {}, outside {}",
                normalize_path(&resolved),
                normalize_path(&root)
            );
        }
        Ok(absolute)
    }

    /// Whether a file with exactly this name, case included, exists.
    pub(super) fn exists_exactly(&self, relative: &str) -> Result<bool> {
        let (directory, name) = split_file_name(relative);
        Ok(exact_name_in(&self.listing(directory)?, name))
    }

    fn listing(&self, directory: &str) -> Result<Vec<String>> {
        let absolute = self.paths.project_root.join(directory);
        let entries = match fs::read_dir(&absolute) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read {}", normalize_path(&absolute)));
            }
        };
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                out.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(out)
    }
}

/// Whether a directory listing holds exactly this file name, case included.
/// A case-insensitive file system would open `Foo_bar.wiki` for
/// `Foo_Bar.wiki`; the listing does not.
pub(super) fn exact_name_in(listing: &[String], name: &str) -> bool {
    listing.iter().any(|other| other == name)
}

/// Another name in a directory listing that differs from `name` only in
/// case.
pub(super) fn case_twin_in<'l>(listing: &'l [String], name: &str) -> Option<&'l String> {
    let lower = name.to_lowercase();
    listing
        .iter()
        .find(|other| *other != name && other.to_lowercase() == lower)
}

fn split_file_name(relative: &str) -> (&str, &str) {
    relative.rsplit_once('/').unwrap_or(("", relative))
}

fn namespace_key(name: &str) -> String {
    name.replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn relative_to_root(paths: &ResolvedPaths, path: &Path) -> String {
    path.strip_prefix(&paths.project_root)
        .map(normalize_path)
        .unwrap_or_else(|_| normalize_path(path))
}
