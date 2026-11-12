# Remilia Wiki coverage review

This document adds a coverage step to `prose-review` on this site. Run it after the source and reader-value checks. See [editorial policy](editorial.md) for the rules behind each finding.

## Steps

1. Fetch the Wikipedia article and the Know Your Meme entry for the subject, when they exist. Use `source fetch URL`. Note any that do not exist.
2. List their major topics: the sections and the facts that a reader of either page would expect to find. Merge near-duplicates.
3. Check each topic against the candidate. A topic is covered if the candidate states it, in any section.
4. Check the candidate for undue Remilia weight and for hedged lore.

## Findings

| Finding | Severity |
|---|---|
| A major topic in the Wikipedia or Know Your Meme counterpart is missing from the candidate | P2 |
| Remilia, Milady Maker, Charlotte Fang, or Miya appears in the lead of a non-Remilia article, or `Category:Remilia` is on it | P1 |
| Remilia weight elsewhere in a non-Remilia article: a dedicated section, the closing paragraph, or a mention with no outside source | P2 |
| Lore stated through an attribution frame ("community accounts hold that", "is said to") | P3 |

A missing topic is a gap in research, so the repair is to research it, not to pad. If neither counterpart exists, say so in the review and skip the coverage check.

Cover only topics that sources support. Do not flag a topic that Wikipedia states without a source and that the candidate cannot support either.
