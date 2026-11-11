---
name: wiki-writing
description: Draft or substantially revise sourced encyclopedic MediaWiki prose.
---

# Wiki writing

For CLI operations in an extracted release, use `tools/wikitool/bin/wikitool[.exe]`
from the project root. The runtime and skills are already in place.

Deliver the requested article or revision from inspected sources. Preserve the
subject, exclusions and intended scope; do not replace an adequate brief with
an interview or a useful draft with an unrelated workflow.

Before drafting a new article, check the live wiki for the subject and its
near-duplicates with `source wiki-search TERM --what nearmatch`; `article scout`
reads only the local index and reports `likely_missing` for a page that exists.
Write an off-wiki draft to `.wikitool/drafts/TITLE.wiki`, named as the page
title, and pass `--title` to `article lint` and `review --draft-path`. Content
under `wiki_content/` is the synced mirror, not a drafting area. When an
interview ledger exists, read it and its open items before writing.

Establish support for material claims before asserting them. Keep sources,
human testimony, interpretation and unresolved claims distinct in your working
record; the published voice is a separate decision. Read the site adapter's
guidance before choosing it. Many wikis, especially community, project and
fandom wikis, are a first-party record of their own subject and exist partly
to canonicalize knowledge that is citable nowhere else. There, the contributor
you work with is a source in their own right, whoever they are: what they write or supply as fact (their
own work and roles, the community's operations, and their domain or subcultural
knowledge such as what a term means or how a practice began) goes into the
wiki's voice, corroborated with citations where a source exists, never
attributed to them or weakened for lacking an outside source. Elsewhere,
publish testimony with attribution or hold it as a lead.

Doubt is a question, not a hedge. When a supplied statement is implausible on
its face, contradicts an inspected source or another statement, or alleges
something about an identifiable person or outside organization, ask the contributor
for its basis before writing, then write what is resolved. Inspect local
templates when integration matters; retrieval neighbors do not determine
article structure or approve category membership.

Write in a plain, confident encyclopedic register. Attribution is information:
use it for quotations, opinions, interpretations, disputed claims and
allegations about other people, and frame a single-source account once per
passage rather than in every sentence. Do not hedge established or first-party
facts with "by his account", "reportedly", "has said", "is said to" or similar,
and do not stack qualifiers to sound careful. Prefer concrete actors, actions
and dates over significance language.

Choose detail for the work:
- [Evidence to prose](references/evidence-to-prose.md): research, attribution,
  claim support, quotations and source limits.
- [Human notes](references/human-notes.md): supplied accounts, drafts and
  firsthand knowledge, how they reach the published voice, and private context.
- [MediaWiki structure](references/mediawiki-structure.md): templates, categories,
  references and article integration.
- [Frozen packets](references/frozen-packet.md): only when a supplied evaluation
  explicitly limits inputs and deliverables.

Let the evidence determine structure and length. The lead must reflect the
supported body. Use a claim-source map where it makes support reviewable;
for substantial articles, retain material claims and exact source passages.
A small sourced revision need not manufacture a separate research dossier.

Lint the exact candidate and inspect any mechanical repairs. For a finished new
or substantially revised article, obtain an independent `prose-review` invocation
from the exact candidate and sources, without supplying an intended verdict.
Resolve material findings and re-review substantive changes. If independent review
is unavailable, deliver the completed draft with that limit; self-review cannot
satisfy it.

A draft request ends with the draft and its evidence/review limits. Publication
additionally requires the project's actual human decision on the exact final
bytes and Wikitool's sync workflow. Never invent acceptance or stop an authorized
draft merely because publication is not yet approved.
