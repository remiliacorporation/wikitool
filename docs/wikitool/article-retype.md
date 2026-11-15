# Retyping articles

`wikitool article retype` applies a typing plan to articles. For each page the plan names, it
states the planned classes in the class-template call and removes or adds `[[Category:…]]`
links. It changes nothing else: every edit replaces a parsed node, removes a parsed link with
its line, or inserts text between nodes, and every other byte of the page is kept. A page it
cannot edit that way is refused with the reason and left as it is.

The command needs a site adapter with `[ontology]` that names `class_template`, the vendored
ontology, and both ontology modules (see [Site adapters](site-adapters.md#ontology)). It is a
dry run by default:

```bash
wikitool article retype --plan typing-plan.json --format json
wikitool article retype --plan typing-plan.json --titles-file titles.txt --apply draft
wikitool article retype wiki_content/Main/Wojak.wiki --plan typing-plan.json
```

- `--apply none` (the default) writes nothing and reports the result.
- `--apply draft` writes each changed page to `.wikitool/drafts/`, named like the page's mirror
  file (`Wojak.wiki`, `Old_Wojak.wiki`), and nothing else.

The command never writes `wiki_content/`, contacts no wiki, and records no acceptance. A
retyped draft is published like any other draft: a human accepts its exact bytes with
`article accept` or `article changeset accept`, then `article promote` and `push`.

It reads a page's draft when one exists, and the mirror page otherwise, so a second run after
`--apply draft` starts from the draft. PATH selects one plan item by its mirror file or draft;
`--titles-file` selects several, one title per line; with neither, every plan item runs, in
plan order. The command exits 1 when a page is refused or the project cannot run the typing
checks, after printing the full report.

## The plan

```json
{
  "schema": "wikitool.article-retype-plan.v1",
  "items": [
    {
      "title": "Wojak",
      "classes": ["wiki:InternetMeme"],
      "relations": { "movement": [], "member_of": [] },
      "wikidata": "Q2622436",
      "remove_categories": ["Memes"],
      "add_categories": [],
      "drop_generated": false
    }
  ]
}
```

Unknown keys, another schema, or a title listed twice make the plan unusable. Each item:

| Key | Meaning |
|---|---|
| `title` | The article. Underscores and spaces are equal, and the first letter is case-insensitive. |
| `classes` | The classes the class statement states, primary first. Empty or absent keeps the classes of an existing call and adds no call. |
| `relations` | Relation fields of the class statement. A listed field is set to its values joined with `; `, or removed when its list is empty. A field the plan does not list is kept as written. |
| `wikidata` | The subject's Wikidata item. Absent keeps the call's `wikidata=`; an empty string removes it. |
| `remove_categories` | Category names to unlink, without the `Category:` prefix (it is accepted). A page without a link to one of them is refused. |
| `add_categories` | Category names to link when the page has no link to them. |
| `drop_generated` | Also remove plain links to the categories that the class statement files the page in. Off unless set. |

## What changes

**The class statement.** A page with one class-template call keeps it in place: the planned
classes fill its positional slots, the planned fields are set where they stand, every other
argument is kept byte for byte, and new fields go at the end. A slot or field whose value
already reads as planned (comments and markup aside) is kept as written, so an equivalent call
is not touched. A page without a call gets a new line: after the short description, else
before the first infobox, else at the top of the page. A page whose typed infobox already
states the plan's only class gets no call when the plan sets no field, as the class template's
documentation says; the report notes it.

**Category links.** A removed link goes with its line when nothing but spaces, tabs, and other
removed links share that line; otherwise only the link goes, with the spaces after it when it
starts the line. A new link goes on its own line after the last category link the page keeps,
or at the end of the page after one blank line. Sort keys are never rewritten: a kept link keeps
its sort key, and `drop_generated` keeps a link with a sort key, because the generated link has
none, and notes it. A link to the category page (`[[:Category:…]]`), interwiki links, and every
other link are not category links and are never touched. A page's line breaks, CRLF or LF, are
used for the new lines.

**Generated categories.** The categories a class statement files a page in are read from
`Module:Ontology/display`, as `Module:Infobox` reads them: each class's `classes[id].category`,
or that of its nearest listed ancestor through `subClassOf`. A category marked `small` also
files the page in the nearest ancestor category that is not small.

## Refusals

A refused page is not changed and has no edits. Every refusal is reported:

| Code | When |
|---|---|
| `redirect` | The page is a redirect. |
| `parse_failed` | The page's wikitext does not parse, for example an unclosed template. |
| `category_not_editable` | A link to a category the plan removes, adds, or drops sits inside a template, parameter, or link (`context` `template`, `parameter`, `link`), an `includeonly`, `noinclude`, `onlyinclude`, or `gallery` region, or a comment, `nowiki`, `pre`, `source`, or `syntaxhighlight` region. |
| `class_statement_not_editable` | The class-template call sits in such a place, or the page has more than one (`context` `multiple`). |
| `ontology.unknown_class`, `ontology.renamed_term` | The typing checks of `article lint` report a planned class id unknown or renamed on the result. The message names the current id of a renamed one. |
| `removal_missing` | The plan removes a category the page has no link to. |
| `invalid_plan` | The item contradicts itself or holds markup: a category both removed and added, an added category that `drop_generated` would remove, fields or `wikidata` without classes on a page with no call, or a class id, field value, or category name holding `|`, braces, brackets, or a line break. |
| `page_missing` | The page has neither a draft nor a mirror file. |
| `not_in_plan` | `--titles-file` or PATH names a title the plan does not list. |
| `not_an_article` | The title is outside the article namespace. |
| `rewrite_failed` | The edits cannot be applied as node rewrites. It is a defect if it appears. |

## The report

`--format json` prints one object, schema `wikitool.article-retype.v1`:

| Field | Meaning |
|---|---|
| `schema` | `wikitool.article-retype.v1` |
| `status` | `complete`, or `not_ready` when the project cannot run the typing checks; then `reason` says why and `pages` is empty |
| `reason` | Only with `not_ready` |
| `apply_mode` | `none` or `draft` |
| `plan_path`, `plan_sha256` | The plan file and the SHA-256 of its bytes |
| `ontology_commit` | The vendored ontology's commit |
| `class_template` | The class template, such as `Template:Instance of` |
| `page_count`, `changed_count`, `unchanged_count`, `refused_count` | Pages by status |
| `edit_count`, `drafts_written` | Edits across all pages, and drafts written |
| `pages` | One entry per selected title, in order |

Each page:

| Field | Meaning |
|---|---|
| `title` | The article |
| `status` | `changed`, `unchanged`, or `refused` |
| `source_path`, `source_kind` | The file read and whether it was the `draft` or the `mirror` page; `null` when no file was read |
| `source_sha256`, `result_sha256` | SHA-256 of the source and result bytes; equal unless `changed` |
| `draft_path`, `draft_written` | Where `--apply draft` writes the result, and whether it did |
| `edits` | The edits in source order, below |
| `diff` | A unified diff of the source (`a/` source path) against the result (`b/` draft path); empty unless `changed` |
| `typing` | The typing checks of `article lint` on the result (the source when unchanged or refused): `checks_run`, `skipped` (`check`, `reason`), and `findings` (`code`, `message`, `term`, `via`, `start`, `end`, `evidence`, `remediation`), where `via` is `class_template`, `infobox`, or `module` and the offsets are bytes in the result |
| `refusals` | `code`, `message`, and when known `context`, `category`, `term`, and the byte range `start`, `end` in the source |
| `notes` | What was deliberately left alone, such as a link already present or a kept sort key |

Each edit:

| Field | Meaning |
|---|---|
| `kind` | `class_statement_replace`, `class_statement_insert`, `category_remove`, `category_drop_generated`, or `category_add` |
| `categories` | The categories a category edit is about; one removed line can hold several; absent for a class edit |
| `start`, `end` | The replaced bytes in the source; equal for an insertion |
| `result_start`, `result_end` | The replacing bytes in the result |
| `old`, `new` | The replaced and replacing text |

Offsets are UTF-8 byte offsets. Replacing each edit's `result_start..result_end` in the result
with its `old` text, from the last edit to the first, gives the source exactly. A second run of
the same plan on the result changes nothing; when the plan removes a category, that second run
reports `removal_missing` instead.
