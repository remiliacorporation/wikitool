# Wikitool development

This governs the source checkout. `CLAUDE.md` routes here. Public skills live
in `.agents/skills/`; source `.claude/skills/` files are thin routes.

## Authority

The runtime project is the caller's wiki, not this checkout. Preserve unrelated
work. Source development and disposable fixtures authorize local iteration;
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

Contextmink and Papertiger own their independent setup, receipts and state.
Prefer installed project commands; do not vendor their rendered skills or mutate
a copied worktree database. Wikitool must not initialize companion authority
or opt a project into it implicitly.
