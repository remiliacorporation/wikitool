# Site adapters

A site adapter is a project-relative TOML file. Release overlays include generic and
Remilia Wiki adapters under `tools/wikitool/site_adapters/`. The fresh-release default
configuration selects Remilia Wiki and its adapter together. An existing project config
replaces that configuration in full. To customize a bundled adapter, copy it and select it:

```bash
cp -R tools/wikitool/site_adapters/remilia-wiki site-adapter
wikitool init --adapter-path site-adapter/site-adapter.toml
```

This records the following in `.wikitool/config.toml`:

```toml
[adapter]
path = "site-adapter/site-adapter.toml"
```

The configured path must be project-relative, and its canonical target must remain under the
project root. Absolute paths, `..` escapes, and symlinks to external policy trees are rejected so
snapshots and isolated evaluations cannot depend on mutable files outside the project.

Without this section in an existing project config, Wikitool uses the embedded
`mediawiki-generic` adapter. Only an absent project config inherits the release defaults.

## Machine policy

The current strict document identity begins with:

```toml
schema_version = "site_adapter_v2"
adapter_id = "example-wiki"
docs_profile = "mw-1.44-authoring"
guidance_documents = []
```

`adapter_id` identifies the complete site policy. The contract does not imply adapter inheritance;
it is deliberately distinct from documentation-corpus profiles, authoring-surface profiles, and
projection target profiles.

The adapter can declare:

- short-description and article-quality mechanics;
- required appendices and reference template;
- citation template families and named-reference behavior;
- deterministic exact-host or subdomain rules that request source review;
- infobox preferences and category hints;
- mechanically decidable style constraints and placeholder artifacts;
- target extension and module availability contracts;
- the ontology that `wikitool ontology export` vendors, in an `[ontology]` table;
- relative supplemental guidance documents.

Unknown fields are rejected. Source-review hosts must be normalized lowercase hostnames; URL
substrings, schemes, paths, ports, and wildcards are not accepted as host rules. Guidance paths
must remain within the adapter directory. Wikitool hashes and exposes supplemental Markdown but
never parses it as executable policy.

Source-review rules are routing signals, not universal bans. Their reasons should tell the review skill what to inspect. Semantic exceptions stay in review findings rather than being hidden in substring logic.

Release packaging places the adapter catalog under `tools/wikitool/site_adapters/`.
An optional host bundle goes under its `project/` subdirectory after validating the
policy and copying only declared resources. This supplement remains opt-in; select
it with a project-relative path in `.wikitool/config.toml`.

## Style lint rules

These `[lint]` keys turn on heading and spacing rules from a site's style guide. Each is off
when omitted, so the generic adapter and existing adapters lint as before. A rule set to
`warning` reports a warning; `error` blocks `article changeset accept` until it is fixed.

| Key | Rule ID | Levels | Reports |
|---|---|---|---|
| `heading_leading_article` | `style.heading_leading_article` | `ignore`, `warning` | a heading that begins with "The", "A" or "An" |
| `heading_trailing_punctuation` | `style.heading_trailing_punctuation` | `ignore`, `warning` | a heading that ends in `.`, `,`, `:`, `;`, `!` or `?` outside a quoted title |
| `heading_markup` | `style.heading_markup` | `ignore`, `warning`, `error` | a template, parser function, ref or `<math>` in a heading |
| `heading_repeats_title` | `style.heading_repeats_title` | `ignore`, `warning` | a heading that contains the article title as whole words |
| `consecutive_blank_lines` | `style.consecutive_blank_lines` | `ignore`, `warning`, `error` | two or more blank lines in a row; `article fix` removes the extra lines |
| `lead_paragraph_limit` | `structure.lead_paragraph_limit` | `ignore`, `warning`, `error` | a lead with more prose paragraphs than `max_lead_paragraphs` (default 4) |

Three rules accept only `ignore` or `warning`: a band called "The Beatles", a work titled
"Yahoo!" or an organization whose name contains the article title cannot be told from a style
fault mechanically. Headings and blank lines inside comments and literal tags such as `<pre>`,
`<nowiki>` and `<syntaxhighlight>` are skipped, as are blank lines inside a template or ref.
Internal links in headings are not checked.

Three `[authoring]` preferences gate existing rules: `prefer_sentence_case_headings` gates
`style.sentence_case_heading`, `forbid_markdown` gates `structure.markdown_heading`, and
`require_straight_quotes` enables `style.curly_quotes` as `[lint] forbid_curly_quotes` does.
`prefer_wikitext_only` is recorded in the policy but no rule reads it yet.

## Ontology

A wiki may type its pages with an ontology. `wikitool ontology export --from <dir>` reads a
checkout of the ontology's source repository and vendors it as `vendor/ontology/ontology.json`;
`wikitool ontology generate` turns that copy into `Module:Ontology/data`. Only `export` needs
the source checkout. A wiki that does not use an ontology needs neither command, and lint,
scout, `ontology check` and `ontology report` read the vendored copy only when the adapter has
`[ontology]`.

An optional `[ontology]` table lets `export` run without further flags:

```toml
[ontology]
product = "example-wiki"                   # the wiki's name in product-uses.tsv
source = "https://ontology.example/"       # an identifier recorded as source.ontology
local_prefix = "wiki"                      # recorded with local_namespace as source.local
local_namespace = "https://wiki.example/ns/"
imports = "v1.0.0/imports"                 # product-uses.tsv and pinned extracts
exclude_rename_prefixes = ["legacy"]       # rename subjects left out of the export
```

`imports` is relative to the ontology directory and may not leave it; `--imports` overrides
it, and export fails when neither names it. `exclude_rename_prefixes` is empty by default and
matches the CURIE prefix before the colon exactly, so `legacy` does not exclude `legacyx:`.
`--product` overrides `product`. `source` is only recorded in the export's metadata; Wikitool
never fetches it, so it need not resolve.

`module_sections` narrows `Module:Ontology/data` to the top-level sections the wiki's modules
read:

```toml
module_sections = ["classes", "families", "properties", "relations", "schemes"]
```

Every page that loads the module with `mw.loadData` parses all of it, so sections nothing reads
at runtime still cost parse time and Lua memory on each such page. Mappings and renames are
the large ones. The valid names are `classes`, `families`, `properties`, `relations`,
`schemes`, `mappings`, `renames`, `external` and `source`; an unknown or repeated name is an
adapter error. `source`, which holds the commit and product, is always written whether listed
or not. Omitting the key writes every section, exactly as before. Both `export` and `generate`
write the narrowed module and `generate --check` compares against it, while the vendored copy
always stays complete. Before narrowing, make sure no module, including a function that is
rarely called, indexes a section left out: it would read `nil`.

### Typing checks

With `[ontology]` and the vendored copy, `article lint` checks what an article states about its
subject against the vendored ontology and the wiki's own modules, `Module:Ontology/local` (the
wiki's terms) and `Module:Ontology/display` (field names and kinds of page). Both modules are
data-only Lua tables read from the templates mirror (`templates/ontology/`); a module that runs
code is refused with its line, and a missing one skips the checks with the reason in the lint
report's `ontology.skipped`. These keys say where an article states its typing:

```toml
[ontology]
class_template = "Template:Instance of"     # positional args are class ids; wikidata=Q..
typed_infobox_template = "Template:Infobox" # type= names a kind of page in the display module
date_formats = ["%B %-d, %Y", "%B %Y"]      # strftime; accepted besides ISO 8601

[ontology.infobox_types]
"Template:Infobox person" = "person"        # infoboxes that always render one kind of page
```

The findings are warnings unless `[lint] ontology_typing` says `error` or `ignore`:

| Code | Finding |
|---|---|
| `ontology.unknown_class` | a class-template id, or a kind of page's class, is not a class in either module |
| `ontology.renamed_term` | an id the ontology renamed; the message names the current id |
| `ontology.unknown_field` | a class-template field that the display module does not give a category (other than `wikidata`) |
| `ontology.malformed_wikidata_id` | `wikidata=` is not `Q` and digits |
| `ontology.unknown_type` | an infobox renders a kind of page the display module does not define |
| `ontology.property_not_applicable` | a filled field's property does not apply to the kind's class or its ancestors (`appliesTo`, or a relation's `domainClasses`); a term without either is not reported |
| `ontology.value_type_mismatch` | a value does not parse as its property's `date`, `quantity` (with a unit from `allowedUnits`), `controlled-term` (a value-list member by id or label), `external-id` or `text`; other types are not checked |
| `ontology.unresolved_field_property` | a field's property is in neither module |
| `ontology.unmapped_local_term` | a local term the article uses has no `mappings` entry |
| `ontology.local_shadows_shared` | a local term the article uses is also a shared term |

Values are read as a reader sees them: links as their labels, templates, `<ref>` tags and
comments as nothing, `<br>` and, for a property with cardinality `many`, `;` as separators. A
field counts only where the kind of page lists it.

`wikitool ontology check` runs the module checks over every local term and display field, for
CI, and fails on a finding or a missing module.

`wikitool ontology report` is the evidence for an intake that moves the wiki's terms into the
shared ontology or proposes new properties: the ontology is provisional, and the wiki's pages
show where it fits. It reads every article in the content mirror and reports how many pages use
each local term, each display field without a property with the pages that fill it, the values
that fail their value type grouped by property with example pages, and the class-template ids
that neither module defines.

`article scout` adds an `ontology` object with the subject's probable classes, from the stub,
else the existing page, else the comparable pages, and for each the properties and relation
types that apply with their value types and the fields that carry them.

## Supplemental guidance

Keep target names, relationships, local source rules, category governance,
visual-subject conventions and extension semantics in adapter Markdown. Load
only guidance relevant to the current task. A host supplement must not redefine
retrieval observations as evidence or claim that acceptance authenticates an editor.

Put category definitions and the boundary around taxonomy creation here, not
in a portable skill's hard-coded list. `preferred_categories` contains hints;
it does not enforce a live approved vocabulary. Membership counts, existing
category tags and template catalog families do not imply editorial approval.

All declared guidance contributes to the adapter identity even when an agent
does not need to read every document for a particular task. Changing guidance
invalidates acceptance bound to the previous policy; do not copy old decisions
onto the new identity. A copied adapter remains owned by the consuming project
and does not update automatically when the distribution changes.

## Portability test

A standalone Wikitool checkout with no adapter should initialize offline, expose the generic adapter, and contain no target-wiki URL, relationship, template, category, or source verdict. A host project should regain all intended local behavior only after its explicit adapter is configured.
