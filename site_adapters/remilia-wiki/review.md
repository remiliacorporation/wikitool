# Remilia Wiki review supplement

This document adds Remilia-specific checks and findings to `prose-review` on this site. Run them after the source and reader-value checks. [Review](editorial.md#review) says what review looks for here, and [editorial policy](editorial.md) holds the rules behind each finding.

## Inputs

Give the reviewer the draft's notes (the search log and provenance lines described in [research](research.md#notes)) along with the candidate and its sources. They are evidence of where statements came from, not author rationale.

## What is not a finding

Report nothing that [Review](editorial.md#review) excludes: a claim that is uncited, cited to an informal source, or cited to a source the reviewer could not reach, and contributed content that lacks attribution or a hedge.

## Severity

Findings use the `prose-review` scale and its dispositions:

- P0 stops the work: an acute privacy, fabrication, safety or destructive-operation risk.
- P1 blocks publication (disposition `block`).
- P2 must be fixed before acceptance (disposition `revise`).
- P3 is polish and does not prevent acceptance.

## Coverage check

This check applies to subjects with an outside record ([research](research.md#two-kinds-of-subject)). For a first-party subject, say so in the review and skip it.

1. Fetch the Wikipedia article and the Know Your Meme entry for the subject, when they exist. Use `source fetch URL`. Note any that do not exist.
2. List their major topics: the sections and the facts that a reader of either page would expect to find. Merge near-duplicates.
3. Check each topic against the candidate. A topic is covered if the candidate states it, in any section.

A missing topic is a gap in research. The repair is to research it or ask the contributor, not to pad. If neither counterpart exists, say so in the review and skip the check.

Flag only topics that a source or the contributor's material supports. Do not flag a topic that Wikipedia states without a source when neither a source nor the contributor's material supports it either.

## Policy checks

Check the candidate for Remilia placement ([Article object and framing](editorial.md#article-object-and-framing)), attribution frames on lore ([Lore](editorial.md#lore)), statement origin ([Provenance](editorial.md#provenance)), and the limits in [People](editorial.md#people).

## Findings

| Finding | Severity |
|---|---|
| A private detail covered by [People](editorial.md#people) that the person has not made public and that is not established public record | P0 |
| A claim of misconduct, crime or other reputational harm about an identifiable person or outside organization that neither the contributor nor a document read for the article supplied | P1 |
| Remilia, Milady Maker, Charlotte Fang, or Miya in the lead of an article where Remilia is not part of what the subject is | P1 |
| `Category:Remilia` or one of its subcategories on such an article | P1 |
| A relationship to Remilia placed in a dedicated section or as the closing paragraph | P2 |
| A relationship to Remilia stated without saying what it is ("adjacent"), or Charlotte Fang used as a prestige anchor | P2 |
| A relationship to Remilia that neither a source nor the contributor's material supplies, reported as a question for the contributor | P2 |
| A statement whose origin the notes cannot show, reported as a question for the contributor | P2 |
| A major topic in the Wikipedia or Know Your Meme counterpart is missing from the candidate | P2 |
| Lore stated through an attribution frame ("community accounts hold that", "is said to") | P3 |

A question for the contributor is not a cut and not a hedge. The repair is the contributor's answer, recorded as a provenance line. Cut or change the statement only when it is shown wrong ([Review](editorial.md#review)); a statement shown to be invented is a P0 fabrication.

Citation mismatches, and summaries cited in place of the work they summarize, are graded under `prose-review`'s source-fidelity reference.
