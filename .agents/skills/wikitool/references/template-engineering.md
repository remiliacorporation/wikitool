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

`templates scaffold` uses exact-state plan/apply. Review different existing
bytes before choosing `--overwrite`. Source-wiki HTML and templates are design
evidence, not authority to clone. Migration plans inventory current bytes and
expose ambiguous invocations; no report authorizes bulk transclusion rewrites.
Local zero-use does not establish live retirement readiness.
