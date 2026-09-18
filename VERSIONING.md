# Versioning Policy

This project uses SemVer for human-facing releases and separate schema versions for internal data contracts.

## Canonical release version

Format:

1. `X.Y.Z` for git tags and release notes
2. `X.Y.Z` in Cargo manifests

Artifact naming:

1. `wikitool-X.Y.Z-<target>.zip`
2. Use `--unversioned-names` only for ephemeral CI/non-release artifacts.

## SemVer bump rules

Major (`X`):

1. Breaking CLI contract changes (command removals/renames, incompatible flag behavior)
2. Breaking release artifact contract changes (folder layout, required packaged AI files)
3. Breaking machine-consumed output contract changes used by automation

Minor (`Y`):

1. Backward-compatible command additions
2. Backward-compatible flag additions
3. Backward-compatible release bundle additions

Patch (`Z`):

1. Bug fixes without contract breaks
2. Internal refactors
3. Docs/test/CI fixes

## Pre-1.0 guidance

Current series is `0.y.z`. Before `1.0.0`, breaking changes may happen in minor bumps.

Example: `0.7.0` is a minor bump that intentionally replaces the old `knowledge`, `research`, and
`workflow` command buckets with explicit `catalog`, `article scout`, `source`, and `interview`
surfaces, and changes remote writes to preview/plan/apply contracts.

When CLI and bundle contracts stabilize, cut `1.0.0` and enforce strict SemVer from that point onward.

## Internal schema versioning

Schema versions are independent from SemVer and must be bumped only when their specific contract changes. The versioned families include:

1. `wikitool.skills-manifest.vN` and `wikitool.skills-install.vN`
2. `ai/docs-bundle-vN.json`
3. `site_adapter_vN`
4. catalog, template-catalog, capability, and authoring-surface artifacts
5. `article_scout_vN`, article lint/fix/promote reports, and review changesets
6. `article_acceptance_ledger_vN` and the transactional acceptance-store schema
7. `wiki_interview_vN` and its command/report envelopes
8. the durable sync-store `user_version` and mutation-intent schemas
9. Wikitest scenario, run-receipt, and prose-evidence schemas

Catalog, docs, and other derived retrieval state are intentionally disposable. Current releases surface readiness through manifest-backed `runtime_artifacts` rows and the operator-facing `catalog_generation` contract. Sync baselines, mutation intents and receipts, and article-acceptance decisions are durable authority state: they require explicit, tested migrations and must fail closed when a schema is missing, corrupt, or unsupported. Catalog reset/rebuild operations must preserve those durable stores.

Cutover rule:

1. Do not add compatibility migrations for pre-manifest catalog databases.
2. Never repair a durable authority-store mismatch by deleting or rebuilding it; use a schema-owned migration or stop with a typed diagnostic.
3. Reset and rebuild derived state with `wikitool db reset --yes`, then `wikitool catalog build` or `wikitool catalog warm --docs-profile <PROFILE> --docs-mode missing`.
4. Use `wikitool catalog status --docs-profile <PROFILE>` to verify readiness before relying on local authoring retrieval.

## Release channels

Experimental / top-level steered:

1. Top-level repo can build and run latest submodule state directly:
   `cargo run --manifest-path tools/wikitool/Cargo.toml --package wikitool -- <command>`
2. This channel may include unreleased changes.

Packaged / distributable:

1. Stage the pinned upstream Contextmink and Papertiger packs with `bash scripts/fetch_contextmink.sh --all` and `bash scripts/fetch_papertiger.sh --all`, then use `cargo run --package wikitool --features maintainer -- release build-matrix --contextmink-dist dist/contextmink-dist --papertiger-dist dist/papertiger-dist` from a source checkout to emit per-target zip bundles.
2. Each bundle is a complete agent project. Extract into a new directory and work there. Root AGENTS.md, CLAUDE.md, README and documentation ship alongside native tools under tools/ and complete skills in both harness directories. Fresh projects use tools/wikitool/default-config.toml for Remilia Wiki. Existing project configuration wins. Packaged binaries have no maintainer surface.
3. A project adapter supplement is opt-in via `--host-project-root <PATH>`. It is packaged under
   `site_adapters/project/` and never replaces the public guidance, skills, or built-in catalog.

## Manual release checklist

1. Pick next version using rules above.
2. Update version in `Cargo.toml` workspace package.
3. Move the `[Unreleased]` notes in `CHANGELOG.md` under a dated `## [x.y.z] - date` heading.
4. Run:
   - `cargo build --workspace`
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo test --workspace --all-targets --all-features`
   - `cargo build -p wikitool -p wikitest` (default end-user feature set)
   - `target/debug/wikitest validate`
   - `target/debug/wikitest suite wikitool-regressions --require-all`

The broader capability and prose campaigns are not release gates. Run `wikitest suite wikitool-capabilities --require-all`
when a capability change warrants the full deterministic campaign, and schedule an external
`complex-prose-stress` campaign for substantive authoring/evaluator changes. Packet preparation by
itself is not a pass and must never be used to bless a release.

5. Exercise any changed catalog, authoring or skill surface in an isolated fixture
   or explicitly read-only host. Use the scenarios in `wikitest/TESTING.md`; do
   not reset an operator's real project as a release smoke test. Run docs audit,
   and regenerate the reference only if CLI shape changed. For skill changes,
   build and inspect the distribution and exercise installation plus realistic
   requests. Packaging cannot certify editorial quality.
6. Build release bundles:
   - `bash scripts/fetch_contextmink.sh --platform <platform> --dest dist/contextmink-dist`
   - `bash scripts/fetch_papertiger.sh --platform <platform> --dest dist/papertiger-dist`
   - `cargo run --package wikitool --features maintainer -- release build-matrix --targets <triple> --contextmink-dist dist/contextmink-dist --papertiger-dist dist/papertiger-dist`
   - or run GitHub workflow `.github/workflows/release-artifacts.yml` with `artifact_version=X.Y.Z` for per-platform artifacts
   - every GitHub macOS artifact is explicitly marked unsigned and carries the bounded,
     checksum-first Gatekeeper procedure
7. Run `bash scripts/verify_project_overlay.sh <archive.zip>` against the actual archive. It must contain:
   - root `AGENTS.md`, `CLAUDE.md`, `README.md`, operator documentation, release history and licenses
   - `tools/wikitool/bin/wikitool[.exe]`, `default-config.toml`, adapters and the hash-manifested `skills/` distribution
   - complete `.agents/skills/` and `.claude/skills/` packages for the four public skills and both companions
   - unchanged upstream Contextmink and Papertiger overlays, with native binaries under `tools/<name>/bin/`, manifests, licenses, contracts and pinned `archive.sha256` receipts
   - `tools/wikitool/release-companions.json`, identifying optional companions and their lifecycle owners
   - no wrapper directory, user configuration, content, database, credentials or fabricated install receipt
   Verify fresh use without setup, preservation of existing target and authority bytes on re-extraction, and explicit diagnostics when an optional companion is absent.
8. Verify `SHA256SUMS.txt` matches the uploaded zip assets.
9. Create tag `X.Y.Z`.
