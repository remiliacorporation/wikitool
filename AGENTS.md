# Wikitool

Work from this project directory. The release is ready to use: run
`tools/wikitool/bin/wikitool` (`.exe` on Windows). Remilia Wiki is the default
target; `config show` reports the effective target and policy. No installation,
skill-copying or initialization sequence is required. Read the
[operator guide](docs/wikitool/guide.md) when the task needs it.

`CLAUDE.md` routes here. Select skills by the requested outcome:

- `wikitool`: retrieval, templates, mechanical checks and revision-bound sync.
- `wiki-writing`: new or substantially revised sourced encyclopedic prose.
- `prose-review`: independent editorial assessment.
- `wiki-interview`: missing human knowledge, sources or exclusions.
- `contextmink`: bounded repository discovery and large reads.
- `papertiger`: durable planning and resuming recorded work.

Skills and their references are already present in both harness directories.
A lookup does not need an editorial workflow; an adequate brief does not need
an interview. Complete authorized drafts without inventing publication scope.

## Authority

The project directory owns its configuration, content and runtime state. Preserve
unrelated work. Source development and disposable fixtures authorize local iteration;
they do not authorize live edits, release publication or deployment.

`.wikitool/data/wikitool.db` is derived. Sync revisions and mutation receipts in
`.wikitool/sync/sync.sqlite3`, and decisions in
`.wikitool/acceptance/acceptance.sqlite3`, are durable. Do not delete them to
clear a failure or initialize replacement planning history.

Keep MediaWiki transport and state transitions in code, editorial judgment in
portable skills, site policy in adapters, and harness metadata in its routes.
Retrieval, source support, review, human acceptance and remote mutation are
different outcomes. A recorded name is not authenticated identity.

Use structured parsers or state machines for wikitext, HTML and command contracts.
Preserve explicit JSON contracts and bounds. Maintainer commands stay behind the
`maintainer` feature; default builds are end-user builds.

## Work and verification

Inspect sources before factual writing. Mechanical checks do not establish truth
or editorial acceptance. Substantially revised prose needs independent review;
publication needs the applicable named human's acceptance of the exact bytes.
Never invent acceptance. Inspect scoped mutation plans and apply only within the
user's authorization. Reconcile ambiguous receipts before retrying a remote write.

Contextmink and Papertiger ship as complete upstream packages. Use their bundled
skills and commands directly; where a companion skill refers to an installed
command or binding above an empty slot, the executable under `tools/<name>/bin/`
is that command. Preserve existing planner authority; choose new
history explicitly only when the task needs it. Wikitool never seeds a planner
database, sync baseline or acceptance decision. A fresh project has no planner
authority until `papertiger init` creates one in the project.

On Windows Git Bash, set `MSYS_NO_PATHCONV=1` for arguments that begin with `/`,
such as JSON pointers, so the shell does not rewrite them as paths.

## Source development

When working on Wikitool's Rust source, build with Cargo and use the resulting
`target/debug/wikitool` or `target/release/wikitool` (`.exe` on Windows).
The following checks apply to source changes, not ordinary wiki work.

Choose checks for the changed behavior using [testing](wikitest/TESTING.md).
The local mechanical fixtures have no production access; run them, repair
failures caused by the change, and repeat affected checks within the requested scope.

- Rust behavior: focused tests and `cargo test --workspace`. Maintainer or test
  infrastructure changes also need all-features tests; maintainer/release code
  needs strict Clippy.
- Public CLI behavior: default Wikitool/Wikitest build, `wikitest validate`,
  and `wikitool-regressions --require-all`.
- CLI shape: regenerate the command reference and run docs audit.
- Skills/docs: validate affected links, discovery, packaging and real task routes.
  Run relevant commands to check claims. Structural success is not behavioral proof.

Use [architecture](docs/wikitool/architecture.md) for cross-layer changes,
[skill integration](docs/wikitool/skill-integration.md) for agent guidance,
and [versioning](VERSIONING.md) for releases. These are conditional references,
not startup requirements. Complete implementation and affected verification;
ask only for a missing decision that actually blocks the authorized outcome.
