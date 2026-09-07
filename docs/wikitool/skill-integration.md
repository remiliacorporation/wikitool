# Skill integration

Wikitool follows a four-layer model:

```text
human intent and exact-prose acceptance
                 |
agent skills: interview -> research/write -> independent review
                 |
project site adapter: local machine policy + supplemental guidance
                 |
wikitool core: MediaWiki facts, parsing, evidence artifacts, lint/fix, CAS sync
```

## Why the split matters

Deterministic code should answer questions whose truth is local and testable: which target is configured, which revision was read, whether a template exists, whether wikitext is balanced, which exact bytes were accepted, and whether a write retained its base revision.

Agent procedures should answer questions that require semantic judgment: what sources entail, what weight is proportionate, whether a living-person claim is responsible, what a paragraph contributes, and whether someone would want to read the article.

Project adapters should answer what varies by deployment: citation templates, quality banners, extensions, infobox preferences, categories, source-review signals, terminology, and local editorial context.

Encoding prose doctrine in the binary makes it stale, hard to audit, and falsely authoritative. Encoding revision safety only in prompts makes it optional. Keep each decision at the layer that can actually enforce or reason about it.

## Why there are three editorial skills

Keep routing coarse enough that safety-critical variants cannot be skipped by choosing the wrong sibling skill:

- Human notes are an input mode within `wiki-writing`. The procedure still requires source discrimination, a claim-source map, subject-derived structure, and the same authoring exit conditions.
- Living-person and contentious-claim remediation is a mandatory branch within `prose-review`. It is not an optional alternative to ordinary review.
- `wiki-interview` is separate because interactive intake has its own conversation state, stopping conditions, and neutral ledger artifact.

Create a new top-level skill only when its trigger, procedure, and output authority are genuinely distinct. Scenario-specific rigor belongs in required references and conditional branches when the underlying task is still writing or review.

## Standard flow

1. Select the skill for the requested outcome; resolve only the target and context it needs.
2. The interview skill captures human knowledge and boundaries when needed.
3. The writing skill inspects sources, builds a claim map, and drafts.
4. Wikitool applies deterministic checks and safe mechanical fixes.
5. The prose-review skill independently checks source fidelity, weight, sensitive claims, and reader value.
6. A named human reads the exact final prose.
7. Wikitool records a hash-bound decision, promotes the exact bytes, and performs revision-bound sync review.

No single readiness flag collapses these stages.

## Loading and delivery boundaries

A mechanical lookup does not require authoring, interview, or publication.
An adequate brief does not require another interview. A requested draft ends
at the draft and its disclosed review limits; publishing additionally requires
the applicable exact-prose acceptance and revision-bound sync.

The four canonical entrypoints live under `.agents/skills/`. Conditional
references ship inside each skill package, so installed skills do not depend
on a source checkout's documentation paths. Frozen author and review packets
retain their closed input boundaries through explicit references.

Validate skill distributions with `release build-skills` in a maintainer build
and `skills inspect`. Exercise `skills setup-project --skill-target both` in
a disposable project when changing packaging or references. Packaging success
establishes artifact integrity; it does not demonstrate improved agent judgment.
