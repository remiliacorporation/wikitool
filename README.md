# Wikitool

Wikitool is a MediaWiki CLI for revision-bound editing, source retrieval,
template inspection and deterministic checks. It includes portable skills for
encyclopedic writing, review and human knowledge intake.

The binary contains no LLM. It owns MediaWiki transport, parsing, local catalogs
and durable editing state. Agents judge source support and prose; an explicitly
selected site adapter supplies the wiki's conventions.

## Install and configure

Download the archive and `SHA256SUMS.txt` from the same release, verify the archive,
and unpack it. Add `wikitool` to PATH or invoke its exact path (`wikitool.exe`
on Windows). [Unsigned macOS releases](docs/wikitool/macos-gatekeeper.md) have a
separate trust procedure.

Release archives contain the executable, a hash-manifested `skills/` distribution,
`site_adapters/`, operator docs, licenses, and separately versioned optional
Contextmink and Papertiger packs. Companion setup belongs to those tools; using
Wikitool does not initialize their state.

Install skills in an existing project directory (create the directory first
when starting a new project):

```bash
wikitool skills inspect
wikitool skills setup-project /path/to/project --skill-target auto
```

Setup installs receipt-owned copies and does not edit root instructions.
`auto` detects supported harness markers; use explicit targets, dry-run,
inspection and uninstall through `skills --help`.

Run from the wiki project, or pass `--project-root`:

```bash
wikitool init --wiki-url https://wiki.example.org/ --api-url https://wiki.example.org/api.php
wikitool config show
```

Wikitool has no default target wiki. Read-only source retrieval needs no credentials
or full mirror. Synchronized editing requires an initial `pull --full --all`
baseline. Build catalogs and refresh capabilities only when the task needs them;
see the [operator guide](docs/wikitool/guide.md). Preserve existing local edits.

Writes use `WIKITOOL_BOT_USER` and `WIKITOOL_BOT_PASS` from the project-root
`.env` or process environment; process values win and ancestor files are ignored.
Never commit credentials. The explicit `wiki.mark_edits_as_bot` setting controls
MediaWiki's bot flag, independently of authoring or human review.

## Choose the task

| Outcome | Entry point |
|---|---|
| Wiki reads, templates, mechanical edits and sync | `wikitool` skill and CLI help |
| New or substantially revised encyclopedic prose | `wiki-writing` skill |
| Independent editorial assessment | `prose-review` skill |
| Missing human knowledge, source leads or exclusions | `wiki-interview` skill |

These are conditional lanes, not a mandatory sequence. A parameter lookup does
not require a template migration procedure; an adequate source packet does not
require an interview. Complete an authorized draft and its review without
inventing a publication requirement.

`article scout` returns local observations, not an editorial plan. `article lint`
and mechanical `review` do not establish source fidelity or publishability.
Categories need useful, supported membership under site policy; retrieval
neighbors and missing-category diagnostics do not authorize taxonomy creation.

## Publication boundaries

Changed nonredirect Main pages require a named human's decision bound to exact
content, target and adapter policy before promotion or push. The supplied editor
label is unauthenticated; never invent acceptance. A later byte, target or policy
change invalidates it.

Push and delete preview by default. Inspect the exact scoped plan, then apply
its ID when the user's authorization covers the mutation. Two CLI phases do
not require two human approvals. Existing edits use `baserevid`; creates use
`createonly`. Deletes recheck revisions but retain a check/request race because
MediaWiki has no conditional-delete equivalent.

Never replay an ambiguous write. Inspect and reconcile its durable mutation
receipt; unresolved closure records uncertainty and requires baseline recovery.
The [operator guide](docs/wikitool/guide.md#publication-and-recovery) explains the
route; the `wikitool` skill carries the portable procedure.

## Site policy and state

Without an explicit adapter, Wikitool uses `mediawiki-generic`. Bundled adapters
are inert examples until selected. Copy the desired bundle into the project and
use `init --adapter-path site-adapter/site-adapter.toml`. Unknown fields and
paths escaping the project fail closed. [Site adapters](docs/wikitool/site-adapters.md)
explains typed policy, supplemental prose and packaging.

`.wikitool/data/wikitool.db` is a disposable catalog. The sync store at
`.wikitool/sync/sync.sqlite3` and publication decisions at
`.wikitool/acceptance/acceptance.sqlite3` are durable authority. Diagnose stale
state before choosing recovery; do not delete authority stores or reset the
whole catalog for an ordinary missing result.

Sync supports MediaWiki's `first-letter` title policy. Case-sensitive namespaces
are not supported write targets.

## Develop and evaluate

```bash
cargo build -p wikitool -p wikitest
```

Use [AGENTS.md](AGENTS.md) for source boundaries and
[Wikitest testing](wikitest/TESTING.md) for checks matched to the change.
Skill-only revisions require packaging, reference and realistic routing checks,
not an unrelated runtime rebuild.

Wikitest is source-resident and absent from end-user archives. Mechanical fixtures
can prove exact CLI and state transitions; prepared prose packets do not prove
editorial quality. External author/reviewer work and its limits are recorded
separately. Self-contained hash receipts prove consistency, not authenticity.

## Documentation

- [Operator guide](docs/wikitool/guide.md): task-based use and recovery.
- [Documentation index](docs/wikitool/README.md): specialized contracts.
- [Architecture](docs/wikitool/architecture.md): ownership and durable state.
- [Skill integration](docs/wikitool/skill-integration.md): packaging and behavioral boundaries.
- [Generated reference](docs/wikitool/reference.md): command flags.
- [Versioning](VERSIONING.md) and [changelog](CHANGELOG.md): release policy and history.

Source-development links require a source checkout. For flags in an installed
release, use `wikitool <command> --help`.

AGPL-3.0-only, with supplementary terms in `LICENSE-SSL` and `LICENSE-VPL`.
