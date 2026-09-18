---
name: wikitool
description: Use Wikitool for MediaWiki retrieval, templates, mechanical checks and revision-bound sync.
---

# Wikitool

In an extracted release, invoke `tools/wikitool/bin/wikitool[.exe]` from the
project root. Skills and companions are already present; no setup is needed.
Use `config show` to inspect the shipped default or existing project target.

Use the smallest operation that answers the task. CLI help owns flags. Run from
the project directory or pass `--project-root`. Check `config show` when target identity is unknown
or before a write. Keep credentials out of reports.

To read one live page, use `export PAGE_URL --format wikitext` or
`source fetch PAGE_URL --format wikitext`; `pull` selects categories, templates
or everything, never a single title. To check whether a page exists, use
`source wiki-search TERM --what nearmatch`; some wikis reject `--what title`.
Exported files carry a metadata header; `article lint` reads only paths under
`wiki_content/`, `templates/` or `.wikitool/`, so copy a candidate to
`.wikitool/drafts/TITLE.wiki` and pass `--title`.

For template parameters, start with `templates show NAME --format json --view brief` and follow
its full-view command when all parameters are needed. This is local catalog
evidence and needs `pull --templates` then `templates catalog build` once in a
fresh project. For current remote contracts without a mirror, use
`source mediawiki-templates PAGE_URL --template NAME` with the freshness options
from help. Inspect coverage limits.

`status`, `diff`, `review` and `push` preview all require the global baseline
from one successful `pull --full --all`; a scoped pull reports
`global_baseline_established: false` and does not unlock them. Read-only
retrieval never needs the baseline. Whole-mirror `validate` reports the live
wiki's existing red links and orphans; scope it with `--title` for a change.

Before editing, inspect relevant local changes; include template-scoped status
for template work. Refresh catalogs or capabilities only when missing or stale
for the decision. There is no universal startup sequence. Live pages, local
indexes and generated suggestions have different authority.

Load the procedure that applies:
- [Template engineering](references/template-engineering.md): creation, interface
  changes and migration; a read-only parameter lookup does not need it.
- [Sync and acceptance](references/sync-and-acceptance.md): remote writes,
  target changes and uncertain mutation outcomes.
- [macOS release trust](references/macos-release-trust.md): first-run quarantine
  problems on unsigned macOS releases.

Use the selected adapter for site policy. `wiki-writing` owns substantial prose,
`prose-review` editorial assessment, and `wiki-interview` missing human knowledge.
Mechanical checks and safe fixes do not certify prose or authorize new categories.
Verify suspected missing pages and redirect failures against the intended live API.

Complete the requested scope and report changed behavior, verification and limits.
Preview/apply is one workflow when already authorized. Never replay an ambiguous
write; use the mutation receipt and recovery procedure.
