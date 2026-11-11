# Wiki interview ledger

The Wikitool interview brief is a neutral record, not a hidden prompt or an editorial verdict. Its current sections are:

- **Article Object** — the subject and intended page type.
- **Scope** — inclusions, exclusions, redirects, merges, and splits.
- **Supplied Materials** — received documents and known provenance or access limits.
- **Human Notes** — the person's account, terminology, emphasis, uncertainty, and corrections.
- **Chronology** — dates, ordering, versions, and conflicts.
- **Entities and Relationships** — identified people, projects, groups, terms, related pages, and uncertain identities.
- **Source Leads** — primary or authoritative sources, other leads, and blocking evidence gaps.
- **Exclusions and Holds** — privacy limits, do-not-assert items, disputed claims, and material held pending evidence.
- **Possible Article Shape** — optional human suggestions, not a generated draft plan.

The scout section is tool-written and records local observations. Do not edit it into human testimony or treat it as a required outline.

## Creating a ledger

Before `interview init`, check the live wiki with `source wiki-search TERM --what nearmatch` and read any existing page on the subject. Choose `--intent` and `--source-article` from what exists; the ledger's intent cannot be changed afterwards. Live command help owns the remaining flags.

Keeping testimony apart from inspected sources records provenance; it is not a judgment that the testimony is doubtful. Where site policy makes the wiki a first-party record, note in the ledger that what the person supplied as fact is publishable as fact, and reserve leads for what they are unsure of.

## Recording discipline

Prefer the human's exact terminology in Human Notes, with clarification where needed. Use brief paraphrase only when it preserves meaning. Mark interviewer inference as inference. Never replace an uncertainty with a confident summary.

For each source lead, record enough to find the item: title, author or account, publisher or platform, date or approximate period, URL or archive location, and the claim it may support. A source lead remains a lead until inspected.

For chronology, distinguish exact dates, approximate periods, recollections, publication dates, event dates, and later retrospectives.

For relationships, record direction and confidence. “Worked for,” “collaborated with,” “was discussed by,” “was collected by,” and “is adjacent to” are not interchangeable.

## Open items

Use the sidecar for durable unresolved state. Select the narrowest available kind and preserve source leads and notes. Useful cases include:

- missing or inaccessible evidence;
- rejected low-quality or misidentified sources;
- disproven links;
- unresolved scope;
- privacy exclusions;
- stale interview facts;
- explicit do-not-assert constraints;
- human follow-up needed.

Resolve an item only with a recorded reason. Rejection and deferral are legitimate outcomes and should not disappear.

## Sensitive information

Do not place unnecessary private information in the ledger. If a boundary itself is sensitive, record the minimum operational instruction needed by future agents. Never copy private context into article prose merely because it is present in a local brief.

## Validation limits

`interview validate` checks schema, required sections, timestamps, freshness, sidecar linkage, and structured counts. It does not verify the human's identity, the truth of statements, source reliability, article quality, or publication consent.
