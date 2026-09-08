---
name: wikitool
description: Use Wikitool for MediaWiki retrieval, templates, mechanical checks and revision-bound sync.
---

# Wikitool

Use the smallest operation that answers the task. CLI help owns flags. Run from
the wiki project or pass `--project-root`; the tool's source checkout is not
automatically the runtime. Check `config show` when target identity is unknown
or before a write. Keep credentials out of reports.

For template parameters, start with `templates show NAME --format json --view brief` and follow
its full-view command when all parameters are needed. This is local catalog
evidence. For current remote contracts, use `source mediawiki-templates URL`
with the exact template and freshness options from help. Inspect coverage limits.

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
