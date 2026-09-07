# Wikitool development

This file governs the source checkout. `CLAUDE.md` is a byte-identical mirror;
edit both together. User-facing skills have their sole substantive owner in
`.agents/skills/`; `.claude/skills/` contains thin source-checkout routes.

## Boundaries

- The runtime project root is the caller's wiki, not this source checkout,
  unless a command explicitly accepts a repository root.
- `.wikitool/data/wikitool.db` is a disposable catalog.
  `.wikitool/sync/sync.sqlite3` is durable revision identity; preserve it
  during catalog reset and refresh.
- Use structured parsers or state machines for wikitext, HTML extraction, and
  command contracts. Do not add regex-based parsers.
- Keep JSON output contracts explicit. Hidden maintainer commands stay behind
  the `maintainer` feature; default builds are end-user builds.
- Source work and disposable tests do not authorize live publication. Keep
  target identity, evidence, editorial judgment, human acceptance, and mutation
  receipts distinct.

## Change and verify

Choose the smallest complete change supported by source, specifications, tests,
and observed behavior. Preserve unrelated work. Surface unknown behavior and
failed assumptions; avoid silent fallbacks. Keep naming and documentation
aligned with the final implementation.

For Rust changes, run targeted tests and `cargo test --workspace`. Test
infrastructure or maintainer changes also require `cargo test --workspace
--all-features`. For maintainer code or release machinery, run
`cargo clippy --workspace --all-targets -- -D warnings`.

Public CLI regressions belong in Wikitest. Build default `wikitool` and
`wikitest`, run `wikitest validate`, then the
`wikitool-regressions --require-all` suite. Keep parser, state-machine,
isolation, and receipt-integrity tests beside their code.

CLI contract changes require relevant help checks and reference regeneration:
`cargo run --package wikitool --features maintainer -- docs generate-reference`.
Update affected operator guidance when behavior changes.

Skill-only changes require packaging/manifest validation, usable relative
references, aligned discovery routes, and realistic task-routing checks.
They do not require rebuilding unchanged runtime code or regenerating CLI help.
Disclose when a check validates structure rather than independent behavior.

## Tool and skill ownership

Wikitool skills are portable editorial and mechanical guidance. Site policy
belongs to adapters; harness metadata stays in discovery adapters. Keep narrow
tasks narrow, and put conditional procedures in references with clear triggers.

Contextmink and Papertiger are separately versioned companions. Use their
installed project skills and receipt-owned commands when available; use
environment guidance when no project runtime is installed. Never invent
replacement planning state or mutate a copied worktree database.

Release bundles carry hash-verified companion packs. Fetch scripts stage pinned
releases under `dist/`; staging is not project installation. Setup and uninstall
belong to each tool's own commands. Wikitool must not initialize or mutate
Papertiger authority or opt a project into it implicitly.
