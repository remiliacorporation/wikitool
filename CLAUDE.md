# Wikitool Development Guidance

This file is for implementation work in the wikitool source checkout. `CLAUDE.md` at the repo
root is a byte-identical mirror of this file; edit both together. It is not packaged for users.
User-facing agent procedures live only in the canonical packages under `agent-pack/skills/`.

When changing shipped behavior, update the relevant operator guidance and regenerate
`docs/wikitool/reference.md`. When changing only internal implementation practice, keep the change
scoped here.

## Implementation Rules

- Closely corroborate all implementation work against the authoritative sources for the project:
  specifications, existing code, documentation, tests, and observed runtime behavior.
- Prefer directly evidenced behavior over inferred design.
- When work reveals a canonical or directly evidenced name that supersedes a current label, stage
  that rename across all relevant locations in the same changeset unless a documented blocker
  prevents immediate closeout.
- Implement for correctness first.
- Treat established naming, structure, and subsystem boundaries as evidence, not obligations.
- Preserve them where they aid correctness or comprehension, but not mechanically.
- Where behavior is not directly established, state the uncertainty explicitly, document the gap at
  the relevant site, and do not present hypotheses as facts.
- Do not silently infer missing behavior.
- Do not add defensive code, fallback paths, or error-mitigating logic that obscures divergence from
  the specification or expected behavior.
- Surface errors, mismatches, and unhandled states immediately and locally.
- Prefer explicit assertions, narrow failure points, and observable diagnostics over hidden recovery.
- If the correct behavior at a site is unknown, that unknowing should be visible in the code.
- Write lean, maintainable code with high local comprehensibility.
- Minimize implicit state, cross-file indirection, and abstractions not yet justified by repeated
  evidence.
- Avoid premature generalization.
- Only extract shared machinery when multiple cases demonstrably share the same behavior and
  constraints.
- Use full-cutover judgment where appropriate, but confine changes to what is directly motivated by
  the current work.
- Do not perform speculative rewrites of adjacent code just because it appears improvable.
- If adjacent code is suspect, note it and continue.

## Source Contracts

- Avoid regex-based parsing for wikitext, HTML extraction, and command-contract logic. Use
  deterministic state machines, structured parsers, or character-by-character parsing.
- Keep CLI output contracts explicit. Agent-facing commands should prefer `--format json` when the
  output is consumed programmatically.
- Hidden maintainer commands belong behind the explicit `maintainer` feature; default
  builds are end-user builds.
- The runtime project root is the caller's wiki project, not this source checkout, unless the
  command explicitly accepts a repository root.
- The catalog database at `.wikitool/data/wikitool.db` is disposable. The sync store at
  `.wikitool/sync/sync.sqlite3` is durable revision identity; resets and refreshes must preserve it.

## Verification

- Run targeted unit tests for touched modules.
- Run `cargo test --workspace` before considering source changes complete.
- Run `cargo clippy --workspace --all-targets -- -D warnings` for maintainer-facing cleanup or
  release-adjacent changes.
- For CLI contract changes, run the relevant command help and regenerate
  `docs/wikitool/reference.md` with
  `cargo run --package wikitool --features maintainer -- docs generate-reference`.

## Bounded Output

Before broad or potentially high-output file, text, structured-data, or command-output reads, load `.agents/skills/contextmink/SKILL.md`. Skip known-small direct reads and project-native compact or domain-query commands.

Use `scripts/contextmink` from Bash or the native
`tools/contextmink/bin/contextmink.exe` from Windows PowerShell. Read
`tools/contextmink/agent_integration.md` for the current receipt and command
contract, and `tools/contextmink/README.md` for this checkout's exact source pin
and fresh-clone installation.

Wikitool release bundles independently carry the published, hash-verified
upstream pack pinned by `config/contextmink.*`. The maintainer
`scripts/fetch_contextmink.sh --platform <platform>` stages that pack under
`dist/contextmink-dist/`; it does not select this checkout's developer runtime.

Papertiger is a separately versioned optional planning companion. A release
bundle includes its complete hash-verified upstream pack under `papertiger/`,
but Wikitool never initializes or mutates Papertiger authority. Project setup,
upgrade, skill installation, and uninstall belong to
`papertiger/papertiger(.exe) setup-project|uninstall-project`; preview setup
with `--dry-run --json` and do not opt a project in without an explicit user
decision. Once installed, use the canonical project skill and
`tools/papertiger/agent_integration.md`, not a Wikitool-owned planning wrapper.
