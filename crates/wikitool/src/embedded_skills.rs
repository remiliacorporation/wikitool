//! The public skill procedures compiled into the binary.
//!
//! These are the same source files `release build-skills` copies into a release's
//! `skills/` tree, so `wikitool guide` prints exactly what an installed skill would
//! hold for this build. The drift test keeps this list equal to the canonical
//! `.agents/skills/` sources.

use crate::skills::PUBLIC_SKILL_IDS;

macro_rules! embed_skill_files {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_bytes!(concat!("../../../.agents/skills/", $path)) as &[u8])),*]
    };
}

/// Every embedded file as `(path relative to .agents/skills, exact bytes)`: each
/// public skill's `SKILL.md` and its `references/*.md`.
pub(crate) const EMBEDDED_SKILL_FILES: &[(&str, &[u8])] = embed_skill_files![
    "prose-review/SKILL.md",
    "prose-review/references/blp-sensitive.md",
    "prose-review/references/frozen-packet.md",
    "prose-review/references/reader-value.md",
    "prose-review/references/source-fidelity.md",
    "wiki-writing/SKILL.md",
    "wiki-writing/references/evidence-to-prose.md",
    "wiki-writing/references/frozen-packet.md",
    "wiki-writing/references/human-notes.md",
    "wiki-writing/references/interview-ledger.md",
    "wiki-writing/references/mediawiki-structure.md",
    "wikitool/SKILL.md",
    "wikitool/references/macos-release-trust.md",
    "wikitool/references/sync-and-acceptance.md",
    "wikitool/references/template-engineering.md",
];

/// One embedded procedure a `guide` topic can name.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EmbeddedProcedure {
    /// The skill that owns the file.
    pub(crate) skill: &'static str,
    /// `None` for the skill entrypoint, else the reference's file stem.
    pub(crate) reference: Option<&'static str>,
    pub(crate) bytes: &'static [u8],
}

impl EmbeddedProcedure {
    /// The canonical topic name: `SKILL` or `SKILL/REFERENCE`.
    pub(crate) fn topic(&self) -> String {
        match self.reference {
            Some(reference) => format!("{}/{reference}", self.skill),
            None => self.skill.to_string(),
        }
    }

    /// The first Markdown heading, at any level, as a one-line label.
    pub(crate) fn heading(&self) -> Option<&'static str> {
        let text = std::str::from_utf8(self.bytes).ok()?;
        text.lines()
            .filter(|line| line.starts_with('#'))
            .find_map(|line| line.trim_start_matches('#').strip_prefix(' '))
            .map(str::trim)
    }
}

/// The mechanical skill first, then writing, then review.
const SKILL_DISPLAY_ORDER: [&str; 3] = ["wikitool", "wiki-writing", "prose-review"];

/// Every embedded procedure in display order: each skill's entrypoint, then its
/// references.
pub(crate) fn embedded_procedures() -> Vec<EmbeddedProcedure> {
    let mut out = Vec::with_capacity(EMBEDDED_SKILL_FILES.len());
    for skill in PUBLIC_SKILL_IDS {
        for (path, bytes) in EMBEDDED_SKILL_FILES {
            let Some(rest) = path
                .strip_prefix(skill)
                .and_then(|rest| rest.strip_prefix('/'))
            else {
                continue;
            };
            let reference = if rest == "SKILL.md" {
                None
            } else {
                rest.strip_prefix("references/")
                    .and_then(|name| name.strip_suffix(".md"))
            };
            if rest == "SKILL.md" || reference.is_some() {
                out.push(EmbeddedProcedure {
                    skill,
                    reference,
                    bytes,
                });
            }
        }
    }
    out.sort_by_key(|procedure| {
        (
            SKILL_DISPLAY_ORDER
                .iter()
                .position(|skill| *skill == procedure.skill),
            procedure.reference.is_some(),
        )
    });
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn canonical_skills_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.agents/skills")
    }

    #[test]
    fn embedded_procedures_equal_the_canonical_skill_sources() {
        let root = canonical_skills_root();
        let mut on_disk = BTreeMap::new();
        for skill in PUBLIC_SKILL_IDS {
            let entrypoint = format!("{skill}/SKILL.md");
            on_disk.insert(
                entrypoint.clone(),
                fs::read(root.join(&entrypoint)).expect("read skill entrypoint"),
            );
            let references = root.join(skill).join("references");
            for entry in fs::read_dir(&references).expect("read references") {
                let path = entry.expect("reference entry").path();
                if path.extension().and_then(|value| value.to_str()) != Some("md") {
                    continue;
                }
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                on_disk.insert(
                    format!("{skill}/references/{name}"),
                    fs::read(&path).expect("read reference"),
                );
            }
        }
        let embedded = EMBEDDED_SKILL_FILES
            .iter()
            .map(|(path, bytes)| ((*path).to_string(), bytes.to_vec()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            embedded.keys().collect::<Vec<_>>(),
            on_disk.keys().collect::<Vec<_>>(),
            "embed_skill_files! must list every public SKILL.md and references/*.md"
        );
        assert_eq!(
            embedded, on_disk,
            "embedded skill bytes differ from sources"
        );
    }

    #[test]
    fn every_embedded_file_is_a_named_procedure() {
        let procedures = embedded_procedures();
        assert_eq!(procedures.len(), EMBEDDED_SKILL_FILES.len());
        assert_eq!(procedures[0].topic(), "wikitool");
        assert!(
            procedures
                .iter()
                .any(|procedure| procedure.topic() == "wikitool/sync-and-acceptance")
        );
    }
}
