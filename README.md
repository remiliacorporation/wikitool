# Wikitool

Wikitool is a MediaWiki CLI for revision-bound editing, source retrieval,
template inspection and deterministic checks. It includes portable skills for
encyclopedic writing, review and human knowledge intake.

The binary contains no LLM. It owns MediaWiki transport, parsing, local catalogs
and durable editing state. Agents judge source support and prose; an explicitly
selected site adapter supplies the wiki's conventions.

## Extract and use

Download the archive and `SHA256SUMS.txt` from the same release, verify the archive,
and extract it into a new directory. Open that directory as your agent project.
Invoke `tools/wikitool/bin/wikitool`
(`.exe` on Windows). [Unsigned macOS releases](docs/wikitool/macos-gatekeeper.md) have a
separate trust procedure.

The archive is a complete agent project: root `AGENTS.md`, `CLAUDE.md`, this
README and operator documentation, Wikitool, complete Contextmink and Papertiger
packages, and discoverable skills under `.agents/skills/` and `.claude/skills/`.
No setup command is needed. Wikitool's executable, adapters and hash-manifested
skill distribution live under `tools/wikitool/`.

For upgrades, retain your configuration, content and databases. The archive also
ships project documents; preserve any local changes to those before replacing them.

From the project root:

```bash
tools/wikitool/bin/wikitool config show
tools/wikitool/bin/wikitool skills inspect
tools/wikitool/bin/wikitool companions
```

The fresh-release default is **Remilia Wiki**, with its bundled site adapter.
`tools/wikitool/default-config.toml` supplies defaults only while the conventional
`.wikitool/config.toml` is absent. An existing project configuration replaces the
defaults in full. Runtime directories are created when first needed; no content,
credentials, sync baselines, acceptance decisions or planner history are shipped.
Papertiger still requires an explicit authority choice for genuinely new history.

To select another wiki, write project configuration with the optional `init`
command (or edit `.wikitool/config.toml`). Run from the project or pass `--project-root`:

```bash
wikitool init --wiki-url https://wiki.example.org/ --api-url https://wiki.example.org/api.php
wikitool config show
```

Read-only source retrieval needs no credentials
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

The release defaults select Remilia's adapter. An explicit project config without
an adapter uses `mediawiki-generic`. Use `init --adapter-path` to select another
project-relative adapter. Unknown fields and
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

Companion packages and their generated skills are local release outputs, ignored
by Git. Their source identities and archive checksums live in `config/`; release
builds compose the verified upstream packages without maintaining copied installer
code or contracts in Wikitool.

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
