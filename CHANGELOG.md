# Changelog

All notable changes to wikitool are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/).

The release workflow extracts the section for the requested version and fails if it is missing, so land notes here (staged under Unreleased, then retitled) before dispatching a release. Write one line per paragraph or bullet: GitHub release bodies render every newline as a line break, so hard-wrapped prose comes out ragged.

## [Unreleased]

### Added

- `article scout` checks its topic against the configured wiki and reports `live_existence` in the full and brief JSON views and as a text line: `exists` with the live title and revision ID, `redirect` with its target, `missing`, or `unchecked` with the reason, when the wiki API is not configured, the check fails or `--no-live-check` is passed. `local_state` still describes only the local index. The check is one read with a 10-second limit and no retries; it never fails scout.
- `pull --title TITLE` (repeatable) pulls exactly the named pages into the mirror and records a revision baseline for each. It is a scoped pull: it never establishes the global baseline and writes no incremental checkpoint. A title that does not exist on the wiki is reported with action `missing_remote` and is not an error.
- `wikitool guide [TOPIC]` prints the shipped procedures from the binary, so a harness without skill discovery still gets them and the procedures always match the binary. Bare `guide` prints a short orientation: the three skills and when each applies, the sync-baseline and acceptance rules, every topic, and the selected site adapter's guidance documents. `guide wikitool`, `guide wikitool/sync-and-acceptance` (or a reference name that only one skill owns, such as `sync-and-acceptance`) print the skill file's exact bytes, compiled from `.agents/skills/` at build time. `guide adapter:NAME` prints one of the selected adapter's guidance documents byte for byte. Neither form needs a project.
- `wikitool commands WORDS` finds commands by purpose. Every word must match the command path, its description or a short purpose phrase, and each match is printed with its path, one-line description and an `[agent]` or `[operator]` tag. Operator commands set up or maintain a project, its documentation corpora or its distribution: `init`, `db`, `docs` corpus management, `lsp`, `companions` and `skills`. `--limit` caps the printed list while the count covers every match; `--format json` reports the same entries. Without words it lists every command. It needs no project.
- `wikitool upload` uploads one local media file as a `File:` page through the reviewed preview/apply workflow. The preview reads the local bytes and the target's current file state without credentials and returns a plan ID bound to the target, title, local path, SHA-1, size, observed remote file, same-content titles, description text and summary. Apply recomputes the plan, rejects drift, records a durable receipt before sending one `action=upload` request, and verifies the stored SHA-1. An existing file with different content needs `--replace`; identical content is reported as already present. Upload warnings end the mutation as not applied and are never retried with `ignorewarnings`.
- `mutation list`, `mutation show` and `mutation reconcile` cover upload receipts. Reconciliation settles an uncertain upload by the remote file's SHA-1 history and never replays it. `mutation close` does not accept upload receipts.
- `article lint` can enforce a site style guide's heading and spacing rules: a heading that begins with "The", "A" or "An" (`style.heading_leading_article`), ends in punctuation (`style.heading_trailing_punctuation`), contains a template, ref or math (`style.heading_markup`) or repeats the article title (`style.heading_repeats_title`); two or more blank lines in a row (`style.consecutive_blank_lines`, removable with `article fix`); and a lead with more prose paragraphs than `max_lead_paragraphs` (`structure.lead_paragraph_limit`). Each rule has its own `[lint]` adapter key and is off unless the adapter enables it. The leading-article, trailing-punctuation and repeated-title rules cannot tell a name from a fault, so they accept only `ignore` or `warning`. Headings and blank lines inside comments and literal tags such as `<pre>`, `<nowiki>` and `<syntaxhighlight>` are skipped; links inside headings are not checked.
- `wikitool ontology export --from <dir>` vendors an ontology directory in a git checkout into `vendor/ontology/ontology.json`: the registry (classes, families, properties, relations and value lists, keyed by id), the SSSOM mappings without `sssom:NoTermFound`, the renames, and the labels and definitions of the outside terms that `product-uses.tsv` lists for the wiki, read from the pinned N-Triples extracts with `@en` preferred over other `@en-*` tags and those over untagged text. It records the commit, refuses uncommitted changes and files the commit does not contain, rejects malformed rows and extract lines with their file and line, and regenerates the module.
- `wikitool ontology generate` writes `Module:Ontology/data` (`templates/ontology/Module_Ontology/data.lua`) from the vendored copy as a Lua table for `mw.loadData`. `--check` writes nothing and exits non-zero when the module is out of date. In a project without a vendored copy, `generate` exits non-zero, explains that `export` creates the copy from a checkout of the ontology's source repository and that a wiki without an ontology needs neither command, and reports status `not_ready` in JSON. Both ontology commands take `--format json`, which reports the commit, product and paths, and for `generate` whether the module was up to date. No other command reads the vendored copy.
- A site adapter's optional `[ontology]` table supplies the export settings: `product`, `source` (an identifier recorded in the export, never fetched), `local_prefix`, `local_namespace`, `imports` (the directory with `product-uses.tsv` and the pinned extracts, relative to the ontology directory) and `exclude_rename_prefixes` (CURIE prefixes whose renames are left out). `--product` and `--imports` override the adapter. The Remilia Wiki adapter names its ontology this way.
- `[ontology] module_sections` lists the top-level sections, such as `classes` and `schemes`, that `ontology export` and `ontology generate` write into `Module:Ontology/data`. Every page that loads the module with `mw.loadData` parses all of it, so a wiki can leave out large sections its modules never read, such as `mappings` and `renames`. `source` is always written, the vendored copy stays complete, and `generate --check` compares against the narrowed module. Omitting the key writes every section, byte for byte as before. An unknown or repeated name is an adapter error that lists the valid ones. `ontology generate --format json` reports the written sections in `sections`. The Remilia Wiki adapter does not set the key.
- `article accept --content-sha256 <SHA256>` binds an acceptance to the bytes the human actually read. If the file no longer has that hash when accept runs, for example because another process or editor changed it after reading, accept records nothing, exits non-zero with both hashes in the message, and in JSON reports `success: false` and status `content_mismatch` with `expected_content_sha256` and `current_content_sha256`. The hash covers the raw file bytes with no line-ending normalization; `article lint --format json` reports it as `content_sha256` without recording anything. Without the option, accept records whatever bytes the file holds, as before. `wiki-writing` now passes the hash its final review recorded through this option, so Wikitool enforces the comparison.

### Changed

- `article lint PATH` reads any file, inside or outside the project, instead of refusing paths outside `wiki_content/`, `templates/` and `.wikitool/`. Reading is not a project write. A file under those three roots, the paths `article accept`, promotion and push read, is linted whole under the title its path gives or `--title`, and an export metadata header at its top is an error (`structure.export_header_on_publication_path`), because it would be published as page text. Any other file may be an export: the title comes from `--title`, else the metadata header that `export --format wikitext` writes, else the file's place in the project, and a file outside the project with neither asks for `--title`. Such a header is set aside rather than linted: `content_sha256` covers the page text after it, issue lines still count from the top of the file, and JSON reports an `export_header` object. Every lint report, single or batch, now carries `file_sha256`, the whole-file hash that `article accept --content-sha256` compares; it equals `content_sha256` for every file accept can take. A PATH under `wiki_content/` with one `--title` still lints both, batch selectors keep their `wiki_content/` scope, and `article fix`, which writes, keeps its path scope.

- `status` and `diff` no longer refuse when the global baseline from `pull --full --all` is missing. They compare the titles that `pull --title` or a scoped pull recorded, and every report states its baseline in a new `scope` field: `global`, or `pulled_titles` with the number of baselined titles and the selected local files that have no recorded baseline. Under `pulled_titles` such a file is listed as unbaselined rather than new, because a missing row cannot show that the page is absent from the wiki. The store must still be bound to the configured target. `review`, `push`, `delete`, `upload` and `article --changed` keep requiring the global baseline, and the refusal messages now name both routes.
- The `wiki-interview` skill is retired and Wikitool now ships three skills: `wikitool`, `wiki-writing` and `prose-review`. Asking a person for missing knowledge is part of `wiki-writing` (its human-notes reference), and the interview ledger contract is now `wiki-writing/references/interview-ledger.md`. The `interview` commands are unchanged. `skills setup-project` removes the retired skill's unmodified files from a project that an earlier release installed.
- Writing and review guidance treats a wiki that is a first-party record of its own subject accordingly: what a contributor supplies as fact is stated in the wiki's voice, and needless hedging is a review defect. Allegations about a named person or outside organization keep their attribution in every paragraph that states them. A contributor's verdict on their own work is attributed as opinion.
- `prose-review` records the full SHA-256 of the bytes it reviewed and confirms each quotation against the file before reporting it. `wiki-writing` re-reviews a page edited after review or reports it as unreviewed, and hands the reviewed hash to `article accept --content-sha256`.
- The `wikitool` skill documents that batch selectors cover `wiki_content/` only, that `export --urls-file` writes Markdown only, and how `article changeset prepare` and `accept` bind hashes, lint results and warning decisions.
- The durable sync store moves to schema version 8, which adds the upload receipt table and rewrites nothing else. The first command that opens an existing store migrates it; older Wikitool builds then refuse that store.
- Three `[authoring]` adapter keys that were parsed but never read now gate the rules they name: `prefer_sentence_case_headings` gates `style.sentence_case_heading`, `forbid_markdown` gates `structure.markdown_heading`, and `require_straight_quotes` turns on `style.curly_quotes` as `[lint] forbid_curly_quotes` does. The generic adapter sets `prefer_sentence_case_headings = false`, so it no longer reports `style.sentence_case_heading`. `prefer_wikitext_only` is still unread.
- The Remilia Wiki adapter enables the new heading and spacing rules as warnings, with a lead limit of four paragraphs, and its style guide lists every rule lint enforces.
- A changeset prepared with an earlier build must be prepared again if this build's lint output for an item differs, because `article changeset accept` compares the lint issues recorded at preparation. Recorded acceptances are not re-linted; they stay valid while the adapter's publication policy hash is unchanged. New `[lint]` keys left off are omitted from the hashed policy, so adapters that do not set them keep their hash.
- The Remilia Wiki adapter states its mission as both a general internet-culture wiki and Remilia's first-party record, counts a contributor's own knowledge as a source, and no longer treats a missing citation as a defect. Every uncited statement must still trace to contributor material or a document read for the article, never to model memory.
- Remilia Wiki guidance keeps real relationships to Remilia in the article body rather than the lead, a dedicated section or the closing paragraph, and adds a research playbook, a style guide and a coverage step for review. Subjects with an outside record keep a minimum search floor; first-party subjects start from the contributor's material.
- `wiki-writing` and `prose-review` treat their source-support thresholds as defaults that a site adapter may replace. Limits that are not about sourcing stay unless an adapter removes them by name.
- Editing an adapter's policy changes its publication policy hash, so acceptances recorded under the earlier Remilia Wiki adapter need recording again.

### Fixed

- `pull --full --all` no longer fails to establish the global baseline because a page was kept for its local edits when that page's recorded revision is known to belong to this wiki: its recorded content equals the current remote text, or the recorded revision, fetched by ID, has exactly that content. Such a page counts as covered, and push still checks it against that base revision. A row that cannot be verified, such as one migrated from a store with no target, or an operator-invalidated title, still leaves the baseline unestablished, and the error now names the uncovered pages and how to resolve them. `pull --title` on a page the wiki no longer has says when an earlier baseline was kept and is now stale.
- `adapter inspect` reports each `source_documents[].relative_path` relative to the project root with forward slashes. On Windows, and elsewhere when the project root was spelled through a symlink or `..`, it printed the document's absolute canonical path, such as one beginning `//?/`. The publication policy hash never included these paths and is unchanged.
- `article scout` no longer offers an adapter's preferred infobox as available when the local template catalog lacks it, so a draft is not steered into a `template.unavailable` lint error. Such preferences are listed in the new `local_integration.unavailable_infobox_preferences` field with their subject type, in adapter order, so the writer can fall back to the next applicable preference. The new `infobox_availability` field reads `checked_against_template_catalog`, or `not_checked` when the project has no local templates and preferences are listed unverified. The interview's infobox candidates follow the same check.
- Adapter template titles now match the template catalog as MediaWiki titles do: `_` equals a space and the first letter is case-insensitive. Article scout offers a matched preference under the catalog's spelling, which lint accepts, and the catalog's recommendation tags no longer depend on exact spelling.
- `mutation reconcile` on an edit or delete receipt that `mutation close` already closed now reports status `operator_closed` and exits successfully instead of failing with "unsupported terminal phase/outcome"; it contacts no wiki and leaves the receipt, the closure record and the title's write block unchanged. A receipt resolved before the sync store recorded terminal outcomes reports `outcome_not_recorded`.

## [0.12.0] - 2026-09-29

The text form of `templates show` now defaults to a short brief, matching its existing `--view brief` default. Use `--view full` for the prior expanded text output. JSON briefs include every declared parameter key while keeping detailed parameter cards bounded.

### Changed

- On macOS, root `AGENTS.md` now has the agent check the bundled Wikitool, Contextmink and Papertiger executables for Gatekeeper's download quarantine before the first tool command of a session. If they are flagged, the agent explains why and, with the user's approval, clears exactly those files in one command. External checksum verification is now optional guidance rather than a precondition. The macOS trust declaration is `wikitool.macos-release-trust.v2` and lists `papertiger-mise` as optional.
- The release no longer ships `CLAUDE.md`; Claude Code reads `AGENTS.md` directly. Skills remain in both `.agents/skills/` and `.claude/skills/`.
- Bundled Contextmink 0.16.0 and Papertiger 0.20.0 replace 0.14.0 and 0.16.0. Contextmink's release-owned setup no longer accepts `--replace-managed` and uses newer setup receipt schemas. Papertiger's `note` command now requires `--text`, and file evidence resolution checks project-local files. Existing project configuration and durable planner state remain outside the release archive.

### Fixed

- Assign User pages their MediaWiki namespace ID during sync writes and baseline recovery, and include them in `sync pull --all`.
- Honor `templates show --view brief` in text output and expose the complete declared parameter key list in brief JSON.

## [0.11.0] - 2026-09-18

Extracted archives are now complete agent projects; the old wrapper-directory layout is gone. Existing projects keep their configuration and durable stores when the new archive is extracted over them.

### Changed

- Release archives are complete agent projects with root AGENTS.md, CLAUDE.md, README and operator documentation alongside project-relative `tools/`, `.agents/skills/` and `.claude/skills/` paths. Invoke `tools/wikitool/bin/wikitool[.exe]`; skill setup is no longer required. This replaces the old versioned wrapper directory and root executable layout and requires a minor version bump before publication.
- Fresh extracted releases default to Remilia Wiki and its adapter. Existing `.wikitool/config.toml` replaces these defaults in full; extraction does not overwrite user configuration, content or databases. Archive upgrades replace shipped project documents; preserve local edits to those documents. Runtime directories are created on first use without seeding sync or acceptance state.
- Bundled Contextmink 0.14.0 and Papertiger 0.16.0 retain their complete upstream overlay layouts and discoverable skills. Windows also includes the Contextmink bridge executable and skill. Packaging verifies archive pins and binary digests. Creating or migrating planning authority remains an explicit Papertiger operation.
- Skills and the operator guide name the single-page read path (`export` and `source fetch`), the existence check (`source wiki-search --what nearmatch`), the `.wikitool/drafts/` drafting location, the template-catalog prerequisites, and the global baseline that `status`, `diff`, `review` and push preview require. The interview skill checks the live wiki before creating a ledger and treats only the person's own words as testimony; the review skill sets a depth rule and a written review record. These follow fresh-agent research runs against the extracted archive.

### Fixed

- `review` names the concrete push-preview failure, such as a missing `article accept` decision, in its brief output and exit message instead of a generic sentence.
- `article lint` no longer reports `structure.malformed_heading` for template or parameter lines that end with `=`, such as `{{Reflist|refs=`; a heading missing its opening marker is still reported.
- `templates show` and `templates examples` explain the pull and catalog-build prerequisites when the local template catalog has no entries.

## [0.10.0] - 2026-09-09

Automation consuming render checks must accept `render_check_v3`; MediaWiki test fixtures use `wikitest.mediawiki-fixture.v5`. Remilia adapter policy changes invalidate acceptance bound to earlier policy bytes. Durable sync and acceptance stores retain their existing migration rules.

### Added

- `push --progress` emits advisory JSON Lines on stderr during planning and between applied candidates. The final stdout report and durable mutation receipts remain authoritative; a processed candidate can still have a conflict, error, or ambiguous outcome.
- MediaWiki 1.46 authoring documentation profiles are available, and the Remilia Wiki adapter selects the 1.46 site profile. Profiles select current upstream documentation and observed extensions; they do not pin every upstream document to a historical MediaWiki revision.
- Render checks accept unpublished wikitext with an exact source hash, scoped DOM assertions, and nested-interactive checks. Server HTML evidence explicitly does not attest to browser layout.
- Template migration plans inventory current local source bytes and return hash-bound parameter/template patches. Collisions, dynamic expressions, substitution, and overlapping changes require review; a local plan does not establish live retirement readiness.
- Profiled HTML import supports declared portable infobox layouts and appearance mappings, semantic tabs, and retained video through explicitly configured native templates. Media objects remain bound to their capture inventory; unsupported captions, clip fragments and source types fail visibly.

### Changed

- Release packs include hash-pinned Contextmink 0.11.0 and Papertiger 0.13.0. Both companions retain their own optional installation and state lifecycle. Contextmink JSON consumers must adopt JSON Pointer result paths and `json-select --at` in place of `--array`.
- Documentation and skills now select workflows by the requested outcome instead of requiring session-wide refreshes, repeated interviews or publication steps for draft-only work. Template inspection distinguishes compact suggestions from complete local parameters and current live contracts. Remilia category creation is a separate researched taxonomy scope; adopting the updated adapter changes its policy identity and invalidates article acceptance bound to the previous policy.
- Wikitest owns public CLI integration testing through the portable `wikitool-regressions` suite in Linux and Windows CI. The duplicate Bash/Python harness and separate docs-discovery/profile CLI tests are retired; focused invariant tests and native release/companion checks retain their own authorities. The broader capability and external prose campaigns remain opt-in.
- MediaWiki fixtures use `wikitest.mediawiki-fixture.v5` with a complete `siteinfo_response`, allowing API-error and missing-evidence controls. The runner binds documentation discovery and fetches to the same observed loopback API.
- Evidence hashing uses a fixed 64 KiB read buffer and an optimized SHA-256 dependency in debug/test builds; every verification still re-reads the exact bytes.

### Fixed

- Catalog rows, search indexes, and readiness metadata commit together. A failed rebuild preserves the previous generation instead of leaving new rows with stale readiness. Template catalog refreshes rebuild only their own search index, avoiding repeated article-index work.
- A full pull can refresh current absence after an explicitly closed ambiguous creation, preserving the local candidate and closure receipt. Scoped reads cannot clear this invalidation.
- Render checks preserve intentional literal wikitext in code examples while continuing to report parser-error markup.
- Module documentation subpages no longer appear as executable modules or contribute example functions to the authoring surface. Fresh pulls store standard `/doc` pages as wikitext; existing Lua configuration subpages remain modules.
- Template briefs count declarations from both TemplateData and wikitext, without double-counting shared names or treating caller-only arguments as declared parameters. Lua-backed templates no longer report zero declarations merely because they forward their arguments through Scribunto.
- Wikitest rejects unknown fields inside steps and assertions, validates scenario fixture hashes and schemas before execution, and supports zero-request assertions and total method counts. Docs discovery, adapter profile defaults, catalog readiness, database reset, and documentation removal have structured public-CLI regressions.
- Failed Wikitest suites replay as failed evidence when their observed coverage is incomplete; inspection still rejects forged passes or fabricated coverage. Text reports identify failed scenarios and assertions.

## [0.9.0] - 2026-08-31

### Added

- `import html-to-wikitext` requires a strict hash-bound capture receipt that distinguishes static HTML from rendered DOM and reports bounded CSS and JavaScript resource observations without applying styles or executing scripts.
- The Remilia Wiki `Infobox subject` contract now exercises source-authored `below` content through the live parser.

### Fixed

- The Remilia Wiki `Ambox` render contract now uses an exact published preservation object instead of a placeholder reference that could only produce parser-error markup.

## [0.8.0] - 2026-08-30

### Added

- `skills inspect`, `skills setup-project`, and `skills uninstall-project` validate and install the four public Wikitool skill packages for shared Agent Skills harnesses, Claude, or both. Setup records exact file identities in `.wikitool-skills/project-install.json`; upgrade and uninstall refuse modified, foreign, symlinked, or downgraded state instead of overwriting user work.
- `mediawiki_wikitext` exposes revision-bound source parsing through strict `mediawiki.wikitext-source-profile.v1` and `mediawiki.wikitext-source-document.v1` contracts, verifies exact source bytes before parsing, and returns bounded templates, parameters, classified links, redirects, and protected literal regions for explicit non-overlapping rewrites; corpus acquisition, template expansion, site mappings, and transformation receipts remain caller-owned.
- `wikitest suite wikitool-capabilities --require-all` runs an opt-in deterministic campaign spanning 75 declared capabilities across article state, catalog retrieval and audits, MediaWiki read/sync/mutation behavior, template engineering, knowledge workspaces, imports, companions, and the agent-facing authoring workflow; the manually dispatched `Wikitest capability campaign` retains its receipts with the exact evaluated binaries without making the campaign a release gate.
- `wikitest prose prepare-suite complex-prose-stress` prepares two source-rich long-form authoring assignments for institutional chronology and governance and for a sensitive living-artist biography. Preparation records no demonstrated coverage; both assignments require external authors, claim maps, differently identified reviewers, and held-out evaluation.

### Changed

- Release archives replace the root `AGENTS.md`, `CLAUDE.md`, `.claude/`, `codex_skills/`, `integration/`, and v3 AI-pack manifest with a self-contained `skills/` distribution. `skills/manifest.json` uses `wikitool.skills-manifest.v1` and binds the complete deterministic file inventory; automation that read the old root paths must use `skills/` or `wikitool skills setup-project`.
- The maintainer-only `release build-ai-pack` command is now `release build-skills` and builds only the skills distribution. Release documentation and site-adapter data are staged separately, so skill installation cannot copy unrelated release files into a project.
- Release archives ship a validated built-in adapter catalog containing the generic template and the Remilia Wiki adapter with its reviewed template contracts. Catalog presence does not select an endpoint or policy; explicit project adapters remain opt-in.
- Media reference inventories are now `mediawiki.media-reference-inventory.v2`: every captured object carries its exact bounded source filename, and generated preservation image/audio invocations retain that readable provenance instead of exposing digest-only file links.
- Release bundles update their hash-pinned native companions to Contextmink 0.10.0 and Papertiger 0.11.0. Wikitool verifies each platform archive and embedded source commit while each optional companion retains ownership of its own project lifecycle.
- Project metadata and the default MediaWiki User-Agent identify the canonical `remiliacorporation/wikitool` repository. Wikitool has no default wiki target; projects select MediaWiki endpoints explicitly and keep site-specific behavior in adapters.
- Release dispatch validates its version, changelog, publication flag, and branch once before allocating the native build matrix, while CI runs the independent Wikitest core and prose suites concurrently after one shared build.
- Prose assignments use `wikitest.prose-assignment.v3` with an explicit `focused` or `complex` classification, and prose suites use `wikitest.prose-suite.v2` with an explicit `calibration` or `stress` campaign type. The experimental v2/v1 manifests are no longer accepted.
- Agent author and reviewer submissions use `wikitest.author-submission.v2` and `wikitest.review-submission.v2` and require exact provider, model, harness/version, invocation, access-envelope, and available run-metric metadata under `execution`; prose receipts advance to v7. Human participants must not provide agent execution metadata.
- The shell regression surface is named `tests/cli_integration/` and remains a conventional integration check; Wikitest campaigns are explicitly selected development evaluations and are absent from normal CI and the per-release checklist.

### Fixed

- Release verification now enforces `skills/manifest.json` as the unified skill-distribution authority and rejects the retired AI/agent-pack manifest at archive root.
- Template indexing now distinguishes nested triple-brace parameters from transclusions, preserves complete invocation spans, and omits dynamic parameter expressions from literal example values.
- Wikitest host-read-only isolation now rejects Windows and POSIX absolute or parent-traversal paths consistently on every host, and retained evidence canonicalizes mixed-separator Windows paths without rewriting unrelated POSIX text.
- Wikitest host-read-only snapshots now retain only the runtime config, catalog and durable read stores, wiki corpus, templates, and the configured adapter's declared resources. Unrelated `.wikitool` caches no longer make dogfood copy gigabytes before its bounded scenario begins, and snapshot capture plus drift verification now obey the scenario deadline.
- Prose dogfood assignments now bind the exact packaged `wiki-writing` skill bytes, restoring deterministic packet preparation.
- Web-archive attribute scanning now uses expression-scoped value bounds accepted by current stable Rust without changing quoted or bare attribute extraction.
- Docs imported from schema-v1 precomposed bundles remain searchable as a generic layer under any configured docs profile, while corpora assigned to different non-empty profiles remain isolated.
- The offline CLI compatibility gate now supplies a loopback MediaWiki authority and repository-pin-bound companion fixtures, so sync, capability, and release-package coverage stays network-independent and cannot accidentally validate stale locally staged packs.
- `import cargo --format json` emits one valid JSON document without appending the text-mode policy footer, so machine consumers can parse successful preview and apply results directly.
- Wikitest's local MediaWiki authority supports deterministic `action=parse` and `list=search` requests, and captured canonical workspace paths expand back to the active isolated project when a later command consumes them.

## [0.7.1] - 2026-08-29

### Fixed

- Public architecture and HTML-conversion guidance now describes private producer boundaries generically, and the documentation audit rejects private implementation names before they can enter release archives.
- `release build-matrix` now binds its child Cargo build to the repository target directory it packages, so a caller-level `CARGO_TARGET_DIR` cannot make it silently copy stale binaries from another build root.

## [0.7.0] - 2026-08-29

### Added

- `import html-to-wikitext` exposes deterministic DOM compilation through explicit, independently hashed source and target profiles plus an optional ordered media inventory; source infobox title-row and field-layout semantics are profile data, compiled output stays project-scoped, and JSON reports exact output identity, coverage, media consumption, and unmapped structures without naming a producer or destination wiki.
- macOS GitHub archives are explicitly unsigned and carry checksum-first agent guidance that limits quarantine removal to the exact approved Wikitool, Contextmink, and optional Papertiger executables instead of pretending the program can bypass Gatekeeper or disabling system policy.
- Release bundles include the separately versioned Papertiger 0.10.0 upstream pack as an optional authoring companion, verify its archive and exact source commit, and expose both external packs through `release-companions.json`. Papertiger retains its own setup, upgrade, task authority, skill contract, experimental Mise runner, and uninstall lifecycle; Wikitool works without a project-local installation and never initializes its database.
- `templates closure` exports a bounded, exact named template/module dependency closure with reconciled parameter contracts, exact local file SHA-256 provenance, capability-bound MediaWiki magic-word and parser-function dependencies, known Scribunto runtime libraries, and distinct genuine-missing and dynamically indeterminate dependency records.
- Strict `template_engineering_contract_v1` documents now drive deterministic compatibility checks, parameter-rename and removal diagnostics, conventional documentation headers and footers, live read-only parse-text render-fixture execution, and exact-state-bound scaffold preview/apply without granting automatic transclusion-rewrite authority.
- The strict `site_adapter_v2` boundary gives each MediaWiki project an explicit typed policy and hashed supplemental-guidance layer while the embedded `mediawiki-generic` adapter remains target-neutral; `init --adapter-path` validates and persists an explicit selection, while unknown fields, missing or undeclared adapter files, malformed source-review hosts, and escaping guidance paths fail closed.
- Four substantive public agent skills now own the operator, `wiki-interview`, evidence-to-prose authoring, and independent prose-review workflows; source fidelity, reader value, BLP care, human-notes preservation, and MediaWiki structure are procedures with explicit exit conditions rather than binary prompt strings.
- The durable `.wikitool/acceptance/acceptance.sqlite3` store replaces fixed per-page JSON ledgers and late decision markers as the sole publication authority. `article_acceptance_ledger_v3` payloads bind the self-reported editor claim, timestamp, prose origin, warning decision, exact content SHA-256, normalized API endpoint, and full site-adapter/guidance policy SHA-256; a multi-article decision and all member authorizations commit in one SQLite transaction, preserving prior authority on error or process loss. Legacy JSON ledgers are detected as historical/unbound and never accepted as a parallel authority path; push reports expose verified timestamp/target/policy provenance without adding editor claims to public summaries.
- The separate `wikitest` workspace binary is the executable dogfooding authority for public-CLI mechanics, isolated and host-supplemented local-knowledge retrieval, and real external prose authoring plus blinded independent review. Strict catalogs, bounded subprocesses, coverage closure, v2 receipts bound to the exact evaluator and evaluated binaries, hash-bound artifacts, controlled review oracles, preflighted prose state advances, and recursive inspection make each result replayable without treating lint as prose approval.
- `wiki render-check` now enforces live rendered-HTML contracts for dynamic template and Cargo cutovers, including parser-error and literal-wikilink rejection, exact component counts, interactive-link requirements that exclude crawler-only anchors, required href fragments, required interactive-link classes such as MediaViewer's `mw-file-description`, and exact PageImages/Popups representative files via `--require-page-image`.
- `crates/mediawiki_protocol`, `crates/wikitool_sync`, and `crates/wikitool_publication` make transport, target-bound synchronization, and exact-content publication authority physical crate boundaries instead of sibling modules in the application core.
- Durable target-bound edit and delete mutation receipts distinguish prepared, request-started, response-observed, reconciled, locally advanced, and unresolved phases. `mutation list|show|reconcile|close` exposes recovery without replay; an operator closure records actor, reason, timestamp, and prior phase without claiming an unproved remote outcome, invalidates the title baseline, and requires a fresh authoritative pull.

### Changed

- Wiki capability manifests are `wiki_capabilities_v2` and include normalized siteinfo magic words. Template closure documents and write receipts are v2 so downstream consumers must explicitly accept the runtime-dependency fields and regenerate hash-bound artifacts; old capability manifests remain decodable, but closure export requires refreshed magic-word data.
- The authoring surface is now layered: Rust owns evidence, parsing, lint, revision safety, and ledger interlocks; agent skills own prose and editorial judgment; projects own site-specific adapters; named humans own the exact final publication decision.
- Site policy now uses adapter-domain names throughout: `adapter_id`, `site_adapter_id`, and the canonical `site-adapter.toml` filename replace the older profile terminology. The complete adapter document has no decorative inheritance field; documentation profiles and projection target profiles remain distinct concepts and retain their domain-specific names.
- `article scout` replaces the old `knowledge article-start` front door. Its `article_scout_v1` payload exposes retrieval context and local-integration state without mislabelling neighboring wiki prose as claim evidence; embedded prompt doctrine is absent.
- Knowledge interviews now create neutral ledgers for article object, scope, supplied materials, human notes, chronology, entities, source leads, exclusions, holds, and possible article shape; the agent skill owns conversational questioning and editorial interpretation.
- Release bundles always preserve the public target-neutral guidance and skills. `--host-project-root` may only add a strictly parsed adapter policy and its declared guidance resources under `site_adapter/project/`; undeclared host files cannot ship and host guidance cannot overwrite packaged `CLAUDE.md`, `AGENTS.md`, rules, or skill wrappers.
- MediaWiki's `bot` edit marker is now an explicit `wiki.mark_edits_as_bot` transport policy instead of being sent on every write.
- Local acceptance, promotion, configuration, import, interview, pull, backup, and research-session writes now use same-directory atomic replacement.
- Knowledge readiness now reports `retrieval_ready`, checks the current artifact generation, and exposes docs import failures without claiming that a topic is researched or prose is ready to draft.
- Push and standalone delete now preview by default and require the exact returned plan ID to apply. Existing-page edits use MediaWiki `baserevid`, creates use `createonly`, remotely deleted modified pages require an explicit force-and-createonly recreation, and deletes recheck the exact revision immediately before mutation. Generic mutation retries remain disabled so ambiguous writes are not silently replayed.
- Sync revision identity, incremental checkpoints, and base snapshots now live in the durable `.wikitool/sync/sync.sqlite3` store instead of the disposable knowledge catalog. Existing mixed databases migrate atomically with row-count verification, catalog reset and full-refresh preserve the durable store, and diff/status/review/push fail with typed missing or unestablished-state errors instead of reporting a false zero-change result. Only an explicitly global, successful `pull --full --all` establishes planning authority; scoped pulls and legacy row presence cannot claim coverage they do not prove. Establishment binds the store to the normalized MediaWiki API endpoint, every planning/publication read verifies it, and neither coincident revision IDs nor `--force` can carry identity across wiki targets.
- Research fetches validate and DNS-pin every HTTP redirect against a shared outbound-network policy, cookie matching honors Secure/host-only/path rules, and the v3 cache uses full SHA-256 keys over schema, extractor, user-agent, session fingerprint, and request identity; returned source content also carries a full SHA-256 fingerprint.
- Source-access session import accepts cookie material only from stdin or an existing regular, non-symlink file, so literal cookie headers cannot enter process arguments or be echoed from a rejected selector. Persistence secures and verifies the empty atomic staging file before writing cookie bytes: Windows uses a native protected DACL containing only the current user and supports verbatim long paths, while Unix verifies current-effective-user ownership with directory mode `0700` and file mode `0600`; uncertain permissions fail closed and diagnostics never include cookie values.
- Release bundles consume Contextmink 0.9.2 from upstream archives verified against repository-pinned SHA-256 values and the exact source commit in `contextmink.release-manifest.v1`; its setup lifecycle no longer installs the retired `scripts/contextmink.cmd` diagnostic shim, and Contextmink owns project setup through `setup-project` instead of a second Wikitool installer.
- CI now validates and runs Wikitest's deterministic core and prose-packet suites. The retained shell harness is explicitly subordinate CLI compatibility coverage under `tests/cli_compat/`, and the performance runner lives under `benchmarks/`.

### Removed

- Removed the binary-owned `ai-pack/writing_context/` doctrine layer, the synthetic-phrase and relationship-substring lints, Remilia-specific core defaults, prose-snapshot documentation tests, and executable-ancestor adapter discovery.
- Removed the legacy prose testbench, phrase-oriented fixture pools, hidden expectation files, and repo-only `/test` workflow; Wikitest now owns editorial evaluation protocols and receipts.
- Removed the stale vendored Contextmink 0.6 source tree and `wikitool contextmink` installer. Wikitool no longer carries a fork or source-build fallback for an independent project.
- Removed the public `purge`, `upload`, `move`, `protect`, and `undelete` commands and the incomplete generic write wrappers behind them. Their effects require workflow-specific, target-bound plans and receipts; the only retained purge primitive is a strict library operation used inside wiki-ops' verified Cargo recreation workflow.

## [0.6.1] - 2026-07-07

### Changed

- Release artifact builds now normalize artifact labels to bare semver, emit `SHA256SUMS.txt`, run the packaged `wikitool` binary during CI release verification, and publish checksums with GitHub release assets.
- Release artifact verification now tests `wikitool contextmink install` from the packaged sibling `contextmink/` release pack on Unix and Windows, including the installed Windows bridge path.
- Contextmink setup guidance now explains invocation by active shell and target: Bash-hosted sessions use `scripts/contextmink ...`, Windows PowerShell uses the native `contextmink.exe` for direct contextmink commands, and `contextmink-bridge.exe --script scripts/contextmink ...` is the PowerShell-to-Git-Bash path for the Bash launcher.

### Fixed

- `wikitool contextmink install` now treats the vendored Contextmink source checkout as a first-class install source when wikitool is source-built, and `--from` accepts either a release pack or a source checkout.
- Source-checkout Contextmink installs now build missing `contextmink` binaries during real installs while keeping dry-runs non-mutating and explicit about `source_kind`; dry-run build deferral is limited to missing build outputs, so missing templates or launcher files fail instead of being mislabeled as buildable.
- `docs audit` now treats the live `wikitool contextmink` command as distinct from the retired bare context surface and checks release-note interview framing in `CHANGELOG.md` after the `RELEASE_LOG.md` migration.
- The vendored Contextmink launcher is synced with the canonical 0.6.0 launcher so it can find Cargo in common Windows and WSL layouts instead of relying only on a non-login `PATH`.

## [0.6.0] - 2026-07-06

This release consolidates the latest authoring, review, documentation, and release-bundling work into a public-ready wikitool package. The release artifacts are now self-contained: wikitool builds the vendored Contextmink 0.6.0 source for each target, including the Windows bridge, instead of depending on a post-release fetch script.

### Added

- Extension-tag contracts are now one data model instead of three drifting copies: `writing_context/extensions.md` carries a machine-readable `Contract:` line per content mechanism (tag, parser function, template, module) that the profile overlay parses, the authoring surface consumes (tags gain `documented`, `body_required`, and `attributes`, plus a drift warning when the doc teaches a tag the live wiki does not expose), and lint consumes (every body-required tag contract gains an empty-body check with no tag-specific code; documenting a new tag now documents, surfaces, and lints it in one edit).
- The interview question agenda now flags intent mismatches: an existing page under `--intent new` asks whether the work should be an expansion or a genuinely separate article, instead of proceeding toward a duplicate.
- `knowledge interview init` now runs the local authoring scout itself: it writes a tool-authored `Scout Context` section into the brief (local state, comparable pages, the closest comparable's outline, infobox candidates, categories, citation patterns, missing query terms) and returns an evidence-grounded `question_agenda` - suggested question areas with the evidence that motivates each, for the interviewer to adapt conversationally rather than follow as a script. When the wiki knows nothing the agenda opens with the freeform monologue; when a page or close comparable exists it steers to the delta and the shape question. `--no-scout` starts blank.
- Interview briefs now hand off more than the Draft Plan: `Recommended angle`, `Tone risks`, `Likely misconceptions`, and `Terminology notes` (Editorial Framing), `Blocking evidence gaps` (Research Plan), and `Related wiki pages` (Entities) are parsed as structured signals on the validation summary, and blocking gaps surface as warnings in `article-start --brief-path`.
- `interview validate` warns advisorily when a core section (Article Object, User-Framed Summary, Interview Transcript and Context, Editorial Framing, Research Plan, Draft Plan) is still at its template state, and the summary reports `sections_unfilled`; a freshly-inited blank brief now reads as `warning`, not `valid`. Chronology, Entities, Scope, and Initial Materials never draw fill warnings, per the playbook's do-not-force rule.
- `docs symbols` now carries real signatures and summaries for parser functions, tags, and magic words, extracted from the documenting section (e.g. `#cargo_query` shows a full `{{#cargo_query:tables=...|where=...}}` call instead of the bare name).
- The article-start docs bridge fires on what the draft actually invokes: parser functions (`{{#name:`) and capability-known extension tags detected in the stub now produce docs queries, prioritized ahead of template-derived queries; the old template-only gate is gone.
- `wikitool contextmink install` performs a deterministic contextmink setup in a wikitool project: it finds the pack shipped next to the wikitool binary (or takes `--from`), places the platform binaries, launcher, and guidance templates, generates a wikitool-tailored `.contextmink.toml` (broad scans skip `.wikitool/**`; explicit paths still work), and verifies the installed binary against the pack manifest. Standalone contextmink stays doc-driven because arbitrary repository layouts cannot be assumed; wikitool projects have a known layout, so the install is exact every time. Packaged guidance now routes agents through the installer with `contextmink/SETUP.md` as the manual fallback.
- `wiki cargo tables`, `wiki cargo fields <table>`, and `wiki cargo rows <table>` open the live Cargo lane an authoring agent was blind to: table list, typed field schema (with list markers and delimiters), and filtered row samples (`--field`, `--where`, `--order-by`, `--limit`, `--offset`). Row keys are aliased to match schema field names exactly.
- The article-start brief now carries the closest comparable page's section sequence in document order (`closest_comparable_outline`), template parameter keys inlined on infobox/template/required-template cards, and a `text_preview` on top evidence cards, so drafting starts without extra drill-down calls.
- `wiki surface` now surfaces the live wiki's parser functions (`{{#cargo_query:...}}`-style call hints plus docs queries) — they were synced into the capability manifest but invisible in every curated view — and each local module's source-declared exported functions, which previously only lint could see.
- Template parameter cards now carry TemplateData `example`, `default_value`, `suggested_values`, and `auto_value` where declared; these were parsed and then dropped at the catalog boundary.
- Extension tags in the surface brief carry their wrap syntax and docs query instead of bare names; the 60-entry static HTML-tag allow-list no longer pads every brief payload.

### Fixed

- `wikitool contextmink install` now writes relative to the current project/agent working directory unless `--project-root` is explicit, so a first-run install inside an unmarked directory cannot be captured by an unrelated initialized ancestor.
- Local contextmink staging now skips identical binary installs before copying, so reinstalling through `contextmink-bridge.exe` on Windows no longer trips over its own running executable.
- Blocking evidence gaps recorded in an interview brief now surface as blocking open questions in `article-start --brief-path`, forcing readiness to `not_ready` until resolved or deferred; previously they were only warnings, which readiness ignored - "blocking" did not block.
- Ubiquitous tags (`ref`, `nowiki`, `noinclude`, ...) no longer spend the docs bridge's four capped query slots; slots go to the tags and parser functions an agent actually needs documentation for.
- Extension and technical docs subpage listing passed the namespaced title as `apprefix` while also setting `apnamespace`, which silently matched nothing: every extension corpus was main-page-only and the technical sweeps (`Manual:Hooks/*`, `Manual:$wg*`, `API:*`, `Help:*`) could never enumerate. With the fix the Cargo corpus alone goes from 1 page to 31 pages, 109 sections, and 557 usage examples; re-run `wikitool docs import <Extension>` or `docs update` after upgrading to rehydrate. Redirect-resolved duplicate pages no longer abort an import, and talk-page and archive titles are filtered out of documentation corpora.

### Changed

- The bundled contextmink pin is now `0.6.0`, bringing the release pack up to the current transcript-guard surface (`dirs`, `outline`, `json-select`, `capture`/`run`, and hook-guard helpers) that the ai-pack guidance already teaches.
- Contextmink release staging is now source-owned by wikitool: public release builds compile the vendored `vendor/contextmink` source for each release target, while `--contextmink-dist` remains only an explicit prebuilt-pack override. The release validator rejects platform/binary/bridge mismatches before packaging.
- Every FTS MATCH expression built from arbitrary query text now goes through one shared sanitizing builder: multi-word topics match as all-tokens-as-prefixes with the exact phrase as an OR branch instead of an inert exact phrase, and a double quote in a topic can no longer produce a SQL syntax error (previously it could fail `article-start` outright).
- The consensus section skeleton in `article-start` follows the document order of the closest comparable pages instead of alphabetizing headings.
- Misleading article-start fields renamed to what they are: `EvidenceRef.score` is now `token_estimate` (it was a size, not a rank), `candidate_facts` is now `comparable_page_excerpts` (verbatim prose from other pages, not subject facts), `external_sources_shortlist` is now `citation_template_families` (template type labels, not followable sources), and the subject lane's `summary` is now `top_local_excerpt`.
- The article-start brief drops its `counts` block (it restated the lengths of the arrays beside it), and the pack no longer computes the ~16-query inventory sweep or media summaries that no command ever surfaced.
- Retrieval labels stop overselling: the page-relatedness prior built from a page's structural vocabulary is now called a term profile (`term-profile` retrieval mode and related-page source) instead of "semantic".

### Removed

- Schema autophagy, each item verified against actual read sites: four write-only FTS tables (template examples, module invocations, page references, page media) that were rebuilt on every index pass but never queried; the write-only `docs_links` table and its parser lane; the 14 JSON-list columns on the page term-profile table that duplicated seven relational tables and were never deserialized; `sync_snapshots.content_hash` and `synced_at_unix` (written, never read); the empty `src/index/` module directory. Table renames: `indexed_page_semantics` -> `indexed_page_term_profiles`, `indexed_authoring_contracts` -> `authoring_contracts` (it is catalog-owned, not a page-index child), `knowledge_artifacts` -> `runtime_artifacts` (a cross-module runtime KV store). Existing databases re-bootstrap automatically via the schema fingerprint; run `wikitool knowledge warm` (or `workflow session-refresh`) once after upgrading to repopulate the index.

## [0.5.0] - 2026-07-02

This release is a measured performance pass plus write-API completion. A new benchmark harness recorded a baseline for the heavy CLI lanes, and every optimization shipped here is justified by its before/after on that table; the headline is full-corpus `article lint` dropping from minutes to about a second.

### Added

- Release bundles ship the contextmink transcript guard in a `contextmink/` directory: the pinned release binary (plus `contextmink-bridge.exe` on Windows), instruction templates, and setup docs. contextmink stays a separate binary; nothing routes through wikitool. The pin lives in `config/contextmink.version` and bundles are fetched from contextmink's own GitHub releases at build time.
- `upload`, `purge`, and `move` API commands; `move` preflights the bot account's rights and supports `--no-redirect`, `--move-talk`, `--move-subpages`, and `--dry-run`.
- `protect` and `undelete` API commands with the same bot-rights preflight and `--dry-run` support, completing the write-API family (`watch` was considered and skipped: no wikitool workflow consumes watchlists).
- `wiki cargo count <table>` counts rows in a live Cargo extension table, giving `cargo_count_rows` its CLI surface under a `wiki cargo` group that later Cargo inspection commands can grow into.
- `--format json` on `knowledge inspect stats`, `knowledge inspect orphans`, `knowledge inspect empty-categories`, and `docs list`, closing the agent-facing JSON gaps those siblings already covered.
- `testbench/perf_bench.sh`: a performance harness that times end-to-end CLI scenarios (sync, lint, knowledge, refresh) against a disposable project copy and prints a baseline table; optimizations are held to before/after numbers on it.

### Performance

All numbers measured on the Remilia Wiki corpus (~676 pages, Windows) with `testbench/perf_bench.sh`; medians of repeated runs.

- Batch `article lint` (and the `review` gate's changed-article lint) loads project lint resources once per invocation instead of once per file: full-corpus lint of 395 articles drops from ~275s to ~1.5s with byte-identical findings.
- Database schema DDL and column validation now run once per database, stamped via `PRAGMA user_version`, instead of on every connection open and every sync ledger/snapshot upsert: `status --modified` drops from ~0.76s to ~0.47s and `diff` from ~0.44s to ~0.25s.
- The knowledge index rebuild is skipped when the scanned corpus is byte-identical to the current-generation index (any content, scan-set, or generation change still forces a full rebuild): `knowledge warm` on an unchanged corpus drops from ~1.6s to ~0.6s, and `workflow session-refresh` no longer pays a second full rebuild after pull (~6.1s to ~5.0s warm).
- The disposable runtime database uses `synchronous=NORMAL` under WAL, trimming per-commit fsync stalls on bulk index writes.
- `article fix` no longer reloads lint resources for its post-fix verification pass.

### Changed

- `knowledge build` and `knowledge warm` reports gain an `unchanged` field marking rebuilds that were skipped because the corpus already matched the index.
- Release notes moved from RELEASE_LOG.md to this CHANGELOG.

## [0.4.0] - 2026-06-05

This release adds a structured human-in-loop authoring lane. Wikitool now bundles a `/knowledge-interview` agent skill (for both Claude and Codex; other agents may need to adapt the packaged guidance) and a playbook for shaping an article's intent, scope, and angle before drafting, plus a `knowledge interview` command family that keeps a small ledger of what the interview turned up and what still needs explication or sourcing.

The interview is optional and conversational, not a step you clear before writing, but for real article work it is the normal move after the article-start scout, unless the user opts out or the task is mechanical. 
Its purpose is direction as much as fact intake: even a well-documented subject can need the editor's sense of what belongs on Remilia Wiki, what should be foregrounded, which sources or artifacts matter, and what should not be overstated. 
What the editor says is treated as reasonable truth once it passes a normal editorial check, rather than something that must be laundered through outside secondary coverage before it can be trusted at all. 
Cite a real source when one exists, especially for external, contested, or primary-record claims. 

Unlike the citation standards upheld by e.g. Wikipedia, for Remilia Wiki it is regularly the case that article content is rightfully its own primary source.
While this is not uncommon for subcultural and gaming-related MediaWikis either, it does mean that agent assistance is still beholden to the knowledge you personally provide it with; the interview skill exists to nudge you towards productive knowledge transfer upon the agent, but an article cannot write itself. Don't be lazy!

### New

- `knowledge interview` starts a brief, logs open questions and dead-end sources as open items, resolves them as they close, and validates or audits the ledger. A finished brief feeds `knowledge article-start` and `review` through `--brief-path`.
- The `/knowledge-interview` skill and `interview_playbook.md`, bundled for Claude and Codex.

### Improvements

- The article quality banner now reads as an editorial review state (`unverified`, `wip`, `verified`) rather than an AI-authorship label, and lint leaves an intentional state alone.
- Headings stop tripping over names. e.g. "Place in the Radbro Webring" is left alone because the tool recognizes Radbro and Webring as proper nouns, not Title Case slips.
- Template checks stop flagging parameters a template is visibly used with on real pages.

### Fixes

- Sentence-case heading suggestions keep proper nouns capitalized instead of lowercasing them.
- A page no longer shows as changed over a trailing-newline-only difference during sync.

## [0.3.1] - 2026-05-29

A follow-up to 0.3.0 that makes the wiki target durable and explicit, removes ambiguous overrides and vestigial flags, and tightens the agent-facing command contract. 0.3.0 reorganized the public surface; 0.3.1 makes the tool behave the way that surface implies.

### Breaking changes

- Bare `WIKI_*` environment variables are no longer read. Use `WIKITOOL_WIKI_URL`, `WIKITOOL_WIKI_API_URL`, `WIKITOOL_USER_AGENT`, `WIKITOOL_ARTICLE_PATH`, `WIKITOOL_BOT_USER`, and `WIKITOOL_BOT_PASS`.
- The default docs profile is renamed from `remilia-mw-1.44` to `remilia-wiki`, so the profile no longer pins a MediaWiki version.
- The `knowledge pack` command and the `knowledge article-start` raw-pack flags are removed. Use `knowledge article-start`, `knowledge contracts`, and `knowledge inspect` directly.
- The `--profile` flag is removed from `article lint`, `article fix`, and `review`. It only ever accepted `remilia`; the lint profile is now applied automatically and still reported as `profile_id` on each report.

### Improvements

- `wikitool config show` reports the resolved wiki target with the source of each value (env, config, default, or derived from the API URL) and notes which environment variables apply.
- `wikitool init` writes the Remilia Wiki target by default while runtime resolution stays env > config; `init --no-network` skips namespace discovery for offline bootstrap.
- `delete` gains `--format json`, so every command now has a structured output mode.
- Docs fetches against mediawiki.org send a User-Agent with a project contact URL and honor the upstream `Retry-After` header. Wikimedia began phasing in API rate limits in 2026, and a bare agent is more likely to be throttled.

### Fixes

- `knowledge article-start --view full` no longer emits a `raw_pack_ref` field pointing at the removed `knowledge pack` command.
- `validate --format json` reports findings in the JSON status instead of through a non-zero exit code on expected validation failures.
- Live wiki search snippets decode HTML entities before they reach JSON output.

### Upgrade

- Because the docs profile was renamed, docs imported under `remilia-mw-1.44` are no longer found under `remilia-wiki`. Run `wikitool knowledge warm --docs-profile remilia-wiki --docs-mode missing` (or `wikitool workflow session-refresh`) once to re-hydrate. Fresh installs need no action.

## [0.3.0] - 2026-05-28

A consolidation release. No database reset required — 0.2.0 runtime state carries forward.

0.2.0 introduced the knowledge layer; 0.3.0 builds the workflow around it and sharpens the agent-facing surface.

### New capabilities

- **One-command first-run setup.** `wikitool workflow session-refresh` initializes the local layout, pulls content, warms the knowledge index, and syncs the wiki's capability profile in a single pass. Run it again to refresh a session; use `workflow full-refresh` to rebuild from scratch.
- **Token-efficient `--view brief` outputs.** Compact, interpreted reports for `knowledge article-start`, `knowledge inspect chunks`, `templates`, `wiki surface`, and `review` — less raw retrieval substrate for an agent to wade through.
- **Research session handoff.** `research session` imports browser cookies (raw header, JSON bookmarklet, or Netscape file) so `research fetch` can read session-gated sources.
- **Web archive capture.** `research archive` saves a site's pages and assets to disk, bounded by page count, per-response size, link depth, and a total-bytes budget.

### Fixes

- Fetched content containing en/em dashes (`&ndash;` / `&mdash;`) now decodes correctly instead of emitting garbled characters.
- Access-challenge detection recognizes more anti-bot vendors (Cloudflare, DataDome, Anubis) and no longer flags ordinary pages that merely contain a generic marker such as "captcha".
- Push conflict checking is faster and steadier on large change sets.

### Other

- The source-build feature flag `maintainer-surface` is now simply `maintainer`.
- Substantial internal restructuring for maintainability, with no change to command behavior or output contracts.

## [0.2.0] - 2026-03-18

Breaking release that replaces the retrieval layer with a purpose-built knowledge system for AI-assisted authoring.
Just delete your old wikitool installation ;d

### What changed

The core idea: wikitool's local index should give an AI agent everything it needs to write a good wiki article in one call. 0.1.0 had the raw materials — page chunks, template data, link graphs — but left the agent to assemble them. 0.2.0 introduces `knowledge article-start`, which interprets those materials into an opinionated authoring brief: which sections comparable articles use, which templates and categories apply, what type of subject this is, and where the evidence gaps are.

### Breaking changes

- **Database reset required.** The knowledge index schema is incompatible with 0.1.0. Delete `.wikitool/data/wikitool.db` and rebuild with `wikitool pull --full --all && wikitool knowledge warm --docs-profile remilia-wiki --docs-mode missing`.
- **Removed commands:** `workflow ask`, `workflow authoring-pack`, `db sync`, `index rebuild`. Use `knowledge article-start` and `knowledge build` respectively.
- **Skill surface collapsed.** Nine Claude skills reduced to two: `/wikitool` (operator) and `/review` (content gate). The old skill names no longer resolve.

### New capabilities

**`knowledge article-start`** — The authoring front door. Returns an interpreted brief with:
- Section skeleton derived from comparable page structures, with `content_backed` flags indicating which sections have evidence in the retrieval pack and which need further research
- Subject type hints inferred from infobox usage across comparables
- Template, category, and link surfaces scoped to what similar articles actually use
- Hard constraints from the wiki's profile overlay
- Suggested next actions

**`knowledge build` / `warm` / `status` / `pack`** — Explicit knowledge lifecycle. `warm` builds the content index and hydrates a docs profile in one pass. `status` reports readiness and degradations so agents can tell the difference between missing content, missing docs, and a fully warmed corpus.

**`research wiki-search` / `research fetch`** — External evidence layer. Search the live wiki API and fetch URLs with structured metadata extraction.

**`article lint` / `article fix`** — Wiki-aware draft quality loop. Catches missing short descriptions, citation placement issues, heading case, and structural problems. `fix --apply safe` auto-corrects mechanical issues.

**`wiki profile sync` / `show`** — Live capability inspection. Exposes installed extensions, configured namespaces, and the active Remilia overlay.

**`templates show` / `examples`** — Template catalog. Shows parameters, usage stats, documentation, implementation pages, and real usage examples from the indexed content.

### Improvements

- Section skeleton now extracts headings directly from comparable pages via the content index, rather than relying on whatever headings happened to appear in retrieved chunks. Skeletons for well-covered topics went from 2 sections (Overview + References) to 5-8 meaningful sections.
- Every top-level CLI command now has a help description.
- Agent guidance (`article_structure.md`, `writing_guide.md`) updated to document skeleton interpretation: use as starting point, drop inapplicable sections, investigate `content_backed: false` gaps with `inspect chunks`.
- Retrieval internals split from a monolithic module into focused subsystems: content indexing, references, templates, retrieval, authoring, docs bridge, status.
- Knowledge artifacts are manifest-backed with schema generation tracking, so stale indexes are detected rather than silently degraded.
- Docs bridge enriches authoring retrieval with pinned MediaWiki 1.44 documentation, blending "how MediaWiki says it works" with "how this wiki uses it."
- 146 unit tests (up from 78). CLI regression testbench and acceptance workflow harness expanded.

## [0.1.0] - 2026-02-21

First public release. Single self-contained binary per platform with bundled AI companion pack.

### Features

**Sync & editing** — Pull articles, templates, and categories from any MediaWiki wiki. Push changes with conflict detection, dry-run preview, and edit summaries. Diff, status, and delete with backup support.

**Search & context** — FTS5 full-text search. Cross-page chunk retrieval with token budgeting. Authoring briefs.

**Validation** — Broken link scanning, Lua module linting via Selene, text and JSON report export.

**External wiki tools** — Fetch wikitext or rendered HTML from any MediaWiki site. Export page trees. Bulk import from CSV/JSON.

**Documentation** — Import and search MediaWiki extension docs offline. Offline docs bundle import/export.

**Link analysis** — Backlinks, orphan detection, empty category pruning.

**Inspection** — SEO metatag inspection, network resource analysis.

### Technical

- Rust 2024 edition, bundled SQLite with FTS5, rustls (no OpenSSL)
- 78 unit tests, 36 CLI regression tests
- AGPL-3.0-only with supplementary terms
