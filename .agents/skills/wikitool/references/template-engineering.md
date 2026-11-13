# Template engineering

Start with the exact template or module roots and inspect modified template
bytes with template-scoped status and diff. Inspect target capabilities and
build or refresh the template catalog only as needed for current evidence.

For creation, refactoring, or migration, export `templates closure` for those
roots. Inspect reconciled TemplateData/source/usage parameters, template and
module edges, runtime-provided MediaWiki and Scribunto dependencies, file
hashes, and missing or dynamically unresolved dependencies.

Express the intended interface in a template contract, check it with
`templates contract check`, and run `templates contract render-check` against
the intended target. Inspect fixture results. Semantic assertions establish
HTML structure, text, and attributes; browser testing establishes responsive
and interactive behavior. A read-only parameter lookup does not need a
migration contract or browser session.

Check category behavior separately from visible HTML: inspect the template's
own categories and representative callers, including parameter branches that
can generate membership. Template-only categories belong in `noinclude` or
equivalently scoped documentation; category links in examples need a leading
colon when they should not assign membership. Compare intended before/after
categories through the target API. A successful HTML fixture does not prove
category isolation, and a template migration does not authorize taxonomy expansion.

`templates scaffold` uses exact-state plan/apply. Review different existing
bytes before choosing `--overwrite`. Source-wiki HTML and templates are design
evidence, not authority to clone. Migration plans inventory current bytes and
expose ambiguous invocations; no report authorizes bulk transclusion rewrites.
Local zero-use does not establish live retirement readiness.

When the site adapter has `[ontology]` and the project vendors the ontology,
`article lint` reports `ontology.*` findings (warnings by default) for class
ids, infobox fields whose property does not apply to the page's class, and
values that do not parse as their property's value type, and `article scout`
returns an `ontology` object: the subject's probable classes with the
properties and relation types that apply, their value types, and the infobox
field that carries each. Treat it as the checklist of facts a page of that
kind states. Changes to `Module:Ontology/local` or `Module:Ontology/display`
need `ontology check`; `ontology report` counts how the wiki's pages use local
terms, unmapped fields and failing values. Findings are mechanical: a value
that parses can still be wrong, and a new term needs a mappings entry and the
operator's agreement before it is added.
