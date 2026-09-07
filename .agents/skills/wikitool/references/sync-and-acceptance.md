# Sync and acceptance

Verify the configured API endpoint and intended scope. Durable sync identity
belongs to that endpoint. When deliberately changing targets, preserve the old
store and establish a fresh baseline; do not reuse production identity locally.
Global planning requires a successful explicit `pull --full --all`; scoped or
legacy-migrated rows do not establish coverage.

Before a push, inspect scoped status, diff, and review. Preview the exact titles
and summary, inspect the returned plan, then apply that plan ID with identical
scope, summary, and policy flags when the user's authorization covers the write.
Do not request a second human permission merely because the CLI has two phases.
The tool rejects drift; investigate a rejection instead of silently replanning.

Delete also previews first. Inspect endpoint, title, observed revision, reason,
local effect, and plan ID before applying the exact plan.

## Ambiguous outcomes

Never replay an uncertain write. Inspect `mutation list` and `mutation show`,
then use `mutation reconcile` for the edit or delete. If remote truth remains
permanently unavailable, explicit operator closure records uncertainty rather
than success, invalidates that title's baseline, and requires `pull --full --all`
before another write. Local drift may additionally require deliberate
`--overwrite-local`; a closure is not permission to discard edits.

## Article acceptance

Follow the project's human publication policy. Never invent a human decision
or self-attest to satisfy it. `article accept` binds the exact bytes, target, and
adapter policy to a named decision. A coherent multi-article decision can use
`article changeset prepare` then `article changeset accept`; later byte changes
invalidate individual members. Prose, target, or policy changes invalidate
acceptance. Legacy JSON ledgers cannot authorize publication.

The editor name is self-reported and unauthenticated. Inspect acceptance
provenance in the push report and keep the claimed editor out of public edit
summaries. Retrieval, lint, review, human acceptance, and remote publication
are distinct outcomes. Finish an authorized draft or review without inventing
a requirement to publish it.
