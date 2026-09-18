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
interview ledger exists, read it and its open items before writing; ledger
text under human notes is testimony to attribute, not sourced fact.

Establish support for material claims before asserting them. Keep sources,
human testimony, interpretation and unresolved claims distinct. Use the site's
adapter guidance and inspect local templates when integration matters; retrieval
neighbors do not determine article structure or approve category membership.

Choose detail for the work:
- [Evidence to prose](references/evidence-to-prose.md): research, attribution,
  claim support, quotations and source limits.
- [Human notes](references/human-notes.md): supplied accounts, drafts or private
  context that must survive editing without becoming unsourced fact.
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
