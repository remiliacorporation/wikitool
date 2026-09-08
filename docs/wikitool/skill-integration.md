# Skill integration

Wikitool follows a four-layer model:

```text
human intent and exact-prose acceptance
                 |
agent skills selected by outcome: intake, writing, review or mechanics
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

- Human notes are an input mode within `wiki-writing`, retaining source discrimination, reviewable claim support and the same authoring exit conditions.
- Living-person and contentious-claim remediation is a mandatory branch within `prose-review`. It is not an optional alternative to ordinary review.
- `wiki-interview` is separate because interactive intake has its own conversation state, stopping conditions, and neutral ledger artifact.

Create a new top-level skill only when its trigger, procedure, and output authority are genuinely distinct. Scenario-specific rigor belongs in required references and conditional branches when the underlying task is still writing or review.

## Task boundaries

| Request | Completion |
|---|---|
| Parameter lookup | Interface with local/live evidence and limits; no migration or editorial workflow |
| Draft from adequate sources | Finished sourced candidate, affected mechanical checks and independent review or its disclosed absence |
| Missing human knowledge | Useful intake ledger with unresolved limits; no repeated interview for facts already supplied |
| Editorial assessment | Findings and scoped disposition; no unsolicited rewrite or publication |
| Authorized publication | Exact plan/apply and resulting state verification; reconcile any ambiguous outcome |

Human acceptance applies to publication of Main prose, not to completing a
draft. Preview and apply are two tool phases within already authorized work.
Do not add a human approval stop between them when the same authorization holds.

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

The project directory must already exist. Retain install receipts and verify
both harness copies against the distribution; source-checkout routes and installed
full packages are different delivery modes.
