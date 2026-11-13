---
name: wiki-writing
description: Draft or substantially revise sourced encyclopedic MediaWiki prose, including asking a person for the knowledge it needs.
---

# Wiki writing

For CLI operations in an extracted release, use `tools/wikitool/bin/wikitool[.exe]`
from the project root. The runtime and skills are already in place.

Deliver the requested article or revision from inspected sources. Preserve the
subject, exclusions and intended scope; do not replace an adequate brief with
an interview or a useful draft with an unrelated workflow.

Before drafting a new article, check the live wiki for the subject and its
near-duplicates with `source wiki-search TERM --what nearmatch`. `article scout`
reports the exact title's `live_existence` on the configured wiki, or
`unchecked` and why; its `local_state` reflects only the local index.
Write an off-wiki draft to `.wikitool/drafts/TITLE.wiki`, named as the page
title, and pass `--title` to `article lint` and `review --draft-path`. Content
under `wiki_content/` is the synced mirror, not a drafting area. When an
interview ledger exists, read it and its open items before writing.

Establish support for material claims before asserting them. Keep sources,
human testimony, interpretation and unresolved claims distinct in your working
record; the published voice is a separate decision that site policy makes.
Read the site adapter's guidance, then follow [Human notes](references/human-notes.md)
for how supplied knowledge reaches the page. On a wiki that is a first-party
record of its own subject, what the contributor supplies as fact goes into the
wiki's voice, whoever they are. Elsewhere, publish testimony with attribution
or hold it as a lead. Where the adapter sets its own source-support
thresholds, they replace the defaults in the references.

Doubt is a question, not a hedge. When a supplied statement is implausible on
its face, contradicts an inspected source or another statement, or alleges
something about an identifiable person or outside organization, ask the contributor
for its basis before writing, then write what is resolved. Inspect local
templates when integration matters; retrieval neighbors do not determine
article structure or approve category membership.

Write in a plain, confident encyclopedic register: concrete actors, actions
and dates, no hedge on an established or first-party fact, no stacked
qualifiers. Attribution is information: use it for quotations, opinions,
interpretations, disputed claims and allegations. A single-source account such
as a memoir needs one frame per passage. An allegation about a named person,
such as one pleaded in a lawsuit, keeps its attribution in every paragraph that
states it.

Choose detail for the work:
- [Evidence to prose](references/evidence-to-prose.md): research, attribution,
  claim support, quotations and source limits.
- [Human notes](references/human-notes.md): supplied accounts, drafts and
  firsthand knowledge, asking for what is missing, how it reaches the published
  voice, and private context.
- [MediaWiki structure](references/mediawiki-structure.md): templates, categories,
  references and article integration.
- [Interview ledger](references/interview-ledger.md): a durable record of what a
  person told you, only when it must outlast the session.
- [Frozen packets](references/frozen-packet.md): only when a supplied evaluation
  explicitly limits inputs and deliverables.

Let the evidence determine structure and length. The lead must reflect the
supported body. Use a claim-source map where it makes support reviewable;
for substantial articles, retain material claims and exact source passages.
A small sourced revision need not manufacture a separate research dossier.

Lint the exact candidate and inspect any mechanical repairs. For a finished new
or substantially revised article, obtain an independent `prose-review` invocation
from the exact candidate and sources, without supplying an intended verdict.
Resolve material findings and re-review substantive changes. A page edited
after its review, including a change of voice or attribution, goes back for
re-review or is reported as unreviewed. Hand off for acceptance with the hash
the final review recorded, and pass it as `article accept --content-sha256`
so Wikitool refuses any bytes other than those reviewed. If independent review is
unavailable, deliver the completed draft with that limit; self-review cannot
satisfy it.

A draft request ends with the draft and its evidence/review limits. Publication
additionally requires the project's actual human decision on the exact final
bytes and Wikitool's sync workflow. Never invent acceptance or stop an authorized
draft merely because publication is not yet approved.
