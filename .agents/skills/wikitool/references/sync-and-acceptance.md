# Sync and acceptance

Verify the configured API endpoint and intended scope. Durable sync identity
belongs to that endpoint. When deliberately changing targets, preserve the old
store and establish a fresh baseline; do not reuse production identity locally.
`init` writes a new target without an adapter unless `--adapter-path` names one,
so site policy becomes generic, and it keeps the default `article_path`; check
`config show` and correct the path when the wiki serves pages under `/wiki/$1`.
Global planning requires a successful explicit `pull --full --all`; scoped or
legacy-migrated rows do not establish coverage, and `review` and push preview
refuse until that baseline exists. Read-only `status` and `diff` also run on
the titles that `pull --title TITLE` or a scoped pull recorded; their output
says `scope: pulled_titles` and lists local files without a recorded baseline
as unbaselined, not new.

Before a push, inspect scoped status, diff, and review. Preview the exact titles
and summary, inspect the returned plan, then apply that plan ID with identical
scope, summary, and policy flags when the user's authorization covers the write.
Do not request a second human permission merely because the CLI has two phases.
The tool rejects drift; investigate a rejection instead of silently replanning.
A preview that reports a missing acceptance authorization is asking for the
human decision recorded by `article accept`, not for credentials; credentials
come from `WIKITOOL_BOT_USER` and `WIKITOOL_BOT_PASS` and are checked at apply.

Delete also previews first. Inspect endpoint, title, observed revision, reason,
local effect, and plan ID before applying the exact plan.

`upload --file PATH --filename "Name.jpg" --text-file PATH --summary TEXT`
previews one media upload without credentials. Inspect endpoint, title, SHA-1,
observed remote file, duplicate titles, and plan ID before applying the exact
plan. An existing file with different content needs a planned `--replace`;
identical content is already present and nothing is sent. An upload warning
means nothing was stored: report its keys and do not work around it. Uploads
stay outside `wiki_content/` and the sync baseline.

## Ambiguous outcomes

Never replay an uncertain write. Inspect `mutation list` and `mutation show`,
then use `mutation reconcile` for the edit, delete or upload; an upload settles
by the remote file's SHA-1 and has no operator closure. If remote truth remains
permanently unavailable, explicit operator closure records uncertainty rather
than success, invalidates that title's baseline, and requires `pull --full --all`
before another write. Local drift may additionally require deliberate
`--overwrite-local`; a closure is not permission to discard edits.

For a closed creation whose page is currently absent, retain the local candidate
and run the full pull. Complete enumeration of its namespace refreshes current
absence without claiming what happened to the original request. A scoped pull
does not establish that absence or clear the invalidation.

## Article acceptance

Follow the project's human publication policy. Never invent a human decision
or self-attest to satisfy it. `article accept` binds the exact bytes, target, and
adapter policy to a named decision. Pass `--content-sha256` with the hash of the
bytes the human read, or that the review recorded; accept then refuses, writing
nothing, if the file no longer has that hash. The hash covers the raw file
bytes; `article lint PATH --format json` reports it as `file_sha256` (equal to
`content_sha256` for every file accept can take) without recording anything.
A coherent multi-article decision can use `article changeset prepare` then
`article changeset accept`; later byte changes invalidate individual members. `prepare` freezes each member's hash and lint
result with one truthful `--prose-origin` in the manifest named by `--output`.
`accept` refuses while any member has a lint error and, by default, a warning;
`--warnings accept` records the human's decision to accept warnings. The
manifest's hashes are the bytes to compare with any independent review record
before asking for the decision. Prose, target, or policy changes invalidate
acceptance. Legacy JSON ledgers cannot authorize publication.

The editor name is self-reported and unauthenticated. Inspect acceptance
provenance in the push report and keep the claimed editor out of public edit
summaries. Retrieval, lint, review, human acceptance, and remote publication
are distinct outcomes. Finish an authorized draft or review without inventing
a requirement to publish it.
