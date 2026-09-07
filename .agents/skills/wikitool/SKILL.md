---
name: wikitool
description: Operate Wikitool for MediaWiki retrieval, template inspection, deterministic checks, target configuration, and revision-bound sync. Use for mechanical wiki work; skip infrastructure-only tasks and route prose judgment to the editorial skills.
---

# Use Wikitool

Choose the smallest operation that answers the request. CLI help owns flags.
The runtime root is the caller's wiki project, not the Wikitool source checkout.
Check `config show` when target identity is unknown or before a write; do not
dump configuration or credentials into reports.

Inspect the relevant local changes before modifying content. Article status
does not cover templates: include `status --templates` for template work.
Preserve unrelated edits. Check catalog health or hydrate capabilities when
the task depends on them; a simple lookup needs no universal startup sequence.
State when observations are cached, partial, or live.

Use the active adapter for site-specific policy and terminology. Load its
relevant guidance when making site-specific decisions. Retrieval readiness,
search rank, and neighboring pages identify material to inspect; they do not
establish factual truth or publishability. Verify suspected missing pages and
redirect failures against the target API before proposing repairs.

## Select the lane

- For template creation or migration, read [template engineering](references/template-engineering.md).
- For remote edits, deletion, target changes, or uncertain write outcomes, read
  [sync and acceptance](references/sync-and-acceptance.md) before acting.
- For prose drafting or substantive revision, use `wiki-writing`; for editorial
  assessment, use `prose-review`; for missing human knowledge, use `wiki-interview`.
  An ordinary lookup or syntax fix need not load all three.
- For macOS first-run quarantine problems, read
  [release trust](references/macos-release-trust.md).

`article lint` and safe fixes are mechanical checks. Inspect applied changes;
a passing lint result does not approve prose. Access challenges require an
authorized access path or user help.

Complete the requested scope and report changes, verification, and material
remaining limits. A preview is not a remote write, and a recorded editor name
is not authenticated human identity.
