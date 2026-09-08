# Wikitool operator guide

Use the command that answers the current task. There is no session-start
refresh sequence. CLI help and the [generated reference](reference.md) own
flags; this guide explains when operations are useful and what their results mean.

## Choose an entry point

| Task | Start here | Expand when needed |
|---|---|---|
| Inspect local changes | `status`, `diff`, scoped by title/path | Include `--templates` for templates and modules |
| Read a template interface | `templates show NAME --format json --view brief` | Follow the emitted full-view command for all parameters; `templates examples` for usage |
| Research an article's wiki integration | `article scout TOPIC --view brief` | Follow specific retrieval drilldowns; observations are not approved prose or taxonomy |
| Draft from adequate sources | `wiki-writing` skill | Interview only for missing human knowledge; independent prose review for a completed substantial draft |
| Review article prose | `prose-review` skill | Inspect exact sources; mechanical `review` is a different operation |
| Change templates | [Template engineering](template-engineering.md) | Closure, contract and caller evidence appropriate to the actual change |
| Publish authorized changes | [Publication and recovery](#publication-and-recovery) and the installed `wikitool` skill | Inspect the exact plan before applying; reconcile ambiguous writes |
| Fetch external sources | `source fetch`, `source mediawiki-templates`, `export` | [Access sessions](source-access.md) only for a source challenge |
| Configure site policy | [Site adapters](site-adapters.md) | Select explicitly; policy changes invalidate matching article acceptance |
| Diagnose unavailable retrieval | `catalog status` and the reported missing component | Rebuild only the derived surface that needs it |

A template catalog is local evidence. Its brief may show only suggested parameters;
use the full view for the complete declared interface and usage-only observations.
For a claim about the current remote interface, inspect that source wiki directly
with `source mediawiki-templates URL --template NAME --refresh`. Inspect its
limits and returned source identity. It does not update or authorize the target catalog.

## Initialize only when needed

Run from the intended wiki project, or pass `--project-root`. A source checkout
is not automatically the wiki runtime.

```bash
wikitool init --wiki-url https://wiki.example.org/ --api-url https://wiki.example.org/api.php
wikitool config show
```

Read-only source retrieval needs no credentials or full wiki mirror. For synchronized
editing, establish the required global baseline once with `pull --full --all`.
A scoped or incremental pull does not establish that baseline. Preserve existing
local edits; `--overwrite-local` is an explicit discard decision.

Select a project-owned adapter if the wiki has one. Then build only the local
surfaces needed for the task:

- `catalog build`: local content index.
- `catalog warm --docs-mode missing`: content plus selected documentation readiness.
- `wiki capabilities sync`: stored target capability snapshot.
- `templates catalog build`: local template/interface catalog.

A narrow lookup does not need all four. Refresh a surface when it is missing,
stale for the decision, or rejected by a command. `pull --all` updates local
content; it is not a harmless read-only session ritual.

## Draft and review

Use supplied evidence first. `article scout` supplies local integration facts
when useful; `source wiki-search` searches the configured wiki, not the open web.
External web search and source selection belong to the agent.

```bash
wikitool article scout "Topic" --intent new --format json --view brief
wikitool article lint .wikitool/drafts/Title.wiki --title "Title" --format json
```

The writing skill owns source support and the finished draft. Use `article fix
--apply safe` when its actual repairs are wanted, inspect changes, and repeat
affected checks. Do not run it merely because a recipe includes it. Use
`wiki-interview` only for consequential missing human knowledge or exclusions;
an adequate brief does not require another interview.

For review of an off-wiki draft, `review --draft-path PATH --title TITLE` skips
push preview but may report global catalog diagnostics. Its promotion-oriented
next steps do not require a draft-only task to publish. Independent prose review
and mechanical checks remain separate evidence.

Use existing categories only when their live definition and the sources establish
useful membership. Do not infer approval from neighbors, category observations, or
uncategorized-page counts. Category creation and hierarchy changes are a distinct
taxonomy scope under the site's guidance.

## Publication and recovery

Changed nonredirect Main pages require exact-content human acceptance before
promotion or push. The named editor is an unauthenticated claim, not identity
proof. Never invent that decision. See the `wikitool` skill's sync reference
for acceptance, coherent changesets and recovery.

```bash
wikitool status --title "Title"
wikitool diff --content --title "Title"
wikitool review --title "Title" --summary "Update Title" --format json
wikitool push --title "Title" --summary "Update Title" --format json
# inspect plan_id and apply the same scope, summary and policy flags
wikitool push --title "Title" --summary "Update Title" --apply PLAN_ID
```

Preview/apply is one authorized workflow, not two human approval requests.
Content, target, policy, scope or revision drift invalidates a plan. Investigate
the drift rather than silently replanning. `--force` does not bypass the plan,
target or article-acceptance boundaries.

Existing edits use `baserevid`; creates use `createonly`. Deletes recheck the
revision before requesting deletion, but MediaWiki has no revision-conditional
delete, leaving a check/request race. `delete` also previews before applying.

For an uncertain write, inspect `mutation list`, `mutation show` and
`mutation reconcile`; never repeat the original write to discover its outcome.
Operator closure records unresolved truth, invalidates the title's sync authority,
and requires the documented full-baseline recovery. It is not success or permission
to discard a local candidate.

Verify affected live behavior after publication. Template checks must include
categories on template pages and callers when membership may change. Server-HTML
assertions do not measure browser layout or interaction.

## State and diagnosis

| State | Meaning |
|---|---|
| `.wikitool/config.toml` | Selected target and adapter |
| `.wikitool/data/wikitool.db` | Derived content, docs and capability catalog |
| `.wikitool/sync/sync.sqlite3` | Durable target-bound revisions, snapshots, plans and mutation receipts |
| `.wikitool/acceptance/acceptance.sqlite3` | Durable content/target/policy-bound human decisions |
| `.wikitool/drafts/` | Off-wiki candidates |
| `wiki_content/`, `templates/` | Working sources; preserve local changes |

Inspect a failure before resetting anything. A missing template catalog may need
a catalog build; a source cache miss may need a fetch; neither implies a full
pull or database reset. Use `db reset --yes` only for a diagnosed disposable-catalog
problem or supported migration. It preserves durable authority; never delete those
stores manually to clear a failure. Follow the command's exact recovery instruction.

Writes read `WIKITOOL_BOT_USER` and `WIKITOOL_BOT_PASS` from the selected
project-root `.env` or explicit process environment. Ancestor `.env` files are
ignored; process variables win. Do not print credentials into diagnostics.
`wiki.mark_edits_as_bot` controls transport labeling, independently of who reviewed.

Sync currently supports MediaWiki's `first-letter` title identity, not
case-sensitive namespaces. Unsigned macOS releases have a separate
[trust procedure](macos-gatekeeper.md); a normal Windows or Linux task does not need it.

## Specialized work

- [Template contracts and migrations](template-engineering.md): closure, scaffold,
  exact source patches, rendered behavior and retirement limits.
- [Source access](source-access.md): human-solved challenges and scoped cookie storage.
- [HTML conversion](html-to-wikitext.md) and [source wikitext](source-wikitext.md):
  bounded profiled conversion, distinct from source acquisition and publication.
- [Skill integration](skill-integration.md): portable packages, host policy and
  behavioral evaluation.
- [Architecture](architecture.md): source ownership and durable authority.
- [Testing](../../wikitest/TESTING.md) and [versioning](../../VERSIONING.md):
  source-only development and release workflows; release publication is a separate scope.

For occasional commands such as docs import/search, references, interview open items,
LSP, Cargo import and companion inspection, use their CLI help instead of maintaining
a second command catalog here.
