---
name: wiki-interview
description: Capture missing human knowledge, source leads and exclusions for a wiki article.
---

# Wiki interview

For CLI operations in an extracted release, use `tools/wikitool/bin/wikitool[.exe]`
from the project root. The runtime and skills are already in place.

Produce a useful, neutral intake record when the article needs human knowledge.
Read supplied material and reuse established scope before asking questions.
An adequate brief, source-only research or routine edit does not need an interview.
Before `interview init`, check the live wiki with
`source wiki-search TERM --what nearmatch` and read any existing page on the
subject; choose `--intent` and `--source-article` from what exists, because the
ledger's intent cannot be changed afterwards.

Only the person's own words are testimony. Record answers verbatim under human
notes with who said them and when; never write answers the person did not give,
and label any placeholder or reconstruction as not testimony. `interview
validate` and `interview audit` check structure and staleness only; they cannot
detect fabricated testimony.

Follow the consequential gaps in the person's account: identity, chronology,
firsthand versus secondhand knowledge, source locations, disputed relationships
or exclusions. Do not force a fixed questionnaire. Ask only what materially
changes the article or its research.

Keep testimony, inspected sources, leads and interviewer inference distinct.
That separation records provenance; it is not a judgment that testimony is
doubtful. When site policy makes the wiki a first-party record, what the person
supplies as fact about their own work, the community's operations or their
domain and subcultural knowledge is publishable as fact; note that in the
ledger so authoring states it plainly. Reserve leads for what the person is
unsure of, and ask follow-up questions only where a statement is implausible,
conflicts with a source or another statement, or alleges something about an
identifiable person or outside organization. Do not cross-examine a contributor's
knowledge for lack of outside sources. Preserve corrections and disagreement. Resolve whether ambiguous sensitive
material is private, attribution-limited or excluded; retain only the operational
boundary future work needs, not unnecessary private detail.

For a durable brief, use [the ledger contract](references/interview-ledger.md)
and live command help. `interview init` can supply local scout observations;
they do not become human testimony or a required outline. Read site-adapter
guidance only where it changes the subject's treatment.

Validate the ledger and return its location, useful leads and remaining limits.
Structural validity does not establish truth or publication consent. Continue
into authoring when requested; missing nonessential facts can remain recorded
without preventing a useful bounded draft.
