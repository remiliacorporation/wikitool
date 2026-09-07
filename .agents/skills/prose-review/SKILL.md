---
name: prose-review
description: Independently assess encyclopedic wiki prose for source fidelity, reader value, due weight, and sensitive claims. Use for editorial review or audit; skip mechanical-only checks and prose drafting.
---

# Prose review

Test what the exact article claims against inspected sources and reader needs.
Report actionable findings before the verdict; keep review separate from repair.

For a frozen blinded review packet, first read
[frozen-packet.md](references/frozen-packet.md). Stay within its declared inputs.
For ordinary reviews, identify the candidate bytes or revision and available
sources, and read relevant site-adapter guidance. Disclose sampled or inaccessible
material; it cannot support a complete source-fidelity verdict.

## Independence

A publication-grade review comes from a different invocation or human than the
authoring pass. Supply the candidate, sources, site guidance, and review
procedure without the author's rationale or desired verdict. Participant labels
do not authenticate identity. When independence is unavailable, label the work
self-review; it can identify defects but cannot satisfy independent review.

## Exercise editorial judgment

Read as a reader and reconstruct material claims from the candidate, including
infoboxes, captions, headings, and categories. Test them against the actual cited
documents. Use [source-fidelity.md](references/source-fidelity.md) for entailment,
citation laundering, attribution, and source limits, and
[reader-value.md](references/reader-value.md) for structure, framing, and weight.

Check that dates, actors, quantities, confidence, and causal direction survive
the source-to-prose transformation. A true detail may still be misleading in
placement or emphasis. Relationships to the host wiki need supported importance.
Diagnose concrete failures such as unsupported synthesis, confusing chronology,
or redundant paragraphs; "sounds AI-generated" is not a finding by itself.

Use [blp-sensitive.md](references/blp-sensitive.md) when living-person, health,
crime, drugs, sexuality, identity, harassment, finance, or reputational claims
appear or are implied. Material sensitive claims without adequate support block
publication.

Use appropriate Wikitool mechanical checks for syntax and link diagnostics in
ordinary work. Frozen packets use only supplied observations. Mechanical success
cannot override an editorial defect.

## Report

Give each finding a precise location, evidence, reader impact, and repair direction.
Distinguish verified defects from inference and review limits.

- **P0 / stop:** acute privacy, safety, fabrication, legal, or destructive sync risk.
- **P1 / block:** material source failure, sensitive-claim defect, plagiarism,
  wrong article object, or fundamentally misleading framing.
- **P2 / revise:** significant weight, readability, structure, attribution,
  context, or evidence-coverage defect.
- **P3 / polish:** bounded clarity or consistency issue without distorted meaning.

Then give the reader verdict, source coverage (complete, incomplete, or not
assessable), disposition, and residual risk. Any P0/P1 requires `block`; P2
requires `revise`; P3-only findings may accompany `accept`. If there are no
findings, say so and state the evidence scope. Do not manufacture cosmetic
findings.

A changed candidate invalidates the prior review. Completing review does not
record human acceptance or authorize publication.
