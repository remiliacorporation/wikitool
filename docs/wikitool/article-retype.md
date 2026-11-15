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
- `--apply draft` writes each changed page to its draft under `.wikitool/drafts/`: the draft
  it was read from, or a new one named like the page's mirror file (`Wojak.wiki`,
  `Old_Wojak.wiki`). It writes nothing else.

The command never writes `wiki_content/`, contacts no wiki, and records no acceptance. A
retyped draft is published like any other draft: a human accepts its exact bytes with
`article accept` or `article changeset accept`, then `article promote` and `push`.

It reads a page's draft when one exists, and the mirror page otherwise, so a second run after
`--apply draft` starts from the draft. A draft is any `.wiki` or `.wikitext` file under
`.wikitool/drafts/`, at any depth, whose file name spells the title the way the mirror spells
file names, with spaces or underscores (`Miya Notes.wiki`, `Miya_Notes.wiki`). A title with
more than one draft is refused, and so is a title whose mirror page has a draft, or whose new
draft path already holds a file (such as `Foo--Bar.wiki`, which reads back as the title
`Foo:Bar`), since a result of the mirror page would lose that file's edits.

PATH retypes exactly the file it names: a draft is read and written in place, and a mirror page
is read and written to a new draft, or refused when its title has a draft. `--titles-file`
selects plan items by title, one per line (a byte order mark at the start is ignored); with
neither, every plan item runs, in plan order. Every page is read and retyped before any draft
is written. The command exits 1 when a page is refused or the project cannot run the typing
checks, after printing the full report.

Titles are read as the mirror reads them. A title in a built-in or configured namespace, or in
a namespace folder under `wiki_content/`, such as `Goldenlight:Foo`, is not an article. File
names are compared exactly, case included, so on a case-insensitive file system a title never
reads another title's mirror page, which `pull` stores under a case-safe name. Two titles that
would write one draft (compared without case), or a draft that another file differs from only
in case, are refused. Every path read or written is checked to resolve inside
`.wikitool/drafts/` or `wiki_content/`.

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
| `relations` | Relation fields of the class statement. A listed field is set to its values joined with `; `, or removed when its list is empty. A field the plan does not list is kept as written. A value may not hold wikitext markup. |
| `wikidata` | The subject's Wikidata item, such as `Q42`. Absent keeps the call's `wikidata=`; an empty string removes it. |
| `remove_categories` | Category names to unlink, without the `Category:` prefix (it is accepted). A page without a link to one of them is refused. |
| `add_categories` | Category names to link when the page has no link to them. |
| `drop_generated` | Also remove plain links to the categories that the class statement files the page in. Off unless set. |

## What changes

**The class statement.** A page with one class-template call keeps it in place: the planned
classes fill its positional slots, the planned fields are set where they stand, every other
argument is kept byte for byte, and new fields go at the end. A slot or field whose value
already reads as planned (comments and markup aside) is kept as written, so an equivalent call
is not touched. Numbered slots (`1=`, `2=`) keep their numbers, and new slots after them are
numbered too; a call that numbers its slots out of order, or mixes numbered and unnumbered
slots, is refused. A page without a call gets a new line: after the short description, else
before the first infobox, else at the top of the page. A page whose typed infobox already
states the plan's only class gets no call when the plan sets no field, as the class template's
documentation says; the report notes it.

**Category links.** A removed link goes with its line when nothing but spaces, tabs, and other
removed links share that line; otherwise only the link goes, with one run of the spaces around
it, so no double or trailing space is left. A byte order mark that starts the page stays. A new link goes on its own line after the last category link the page keeps,
or at the end of the page after one blank line. Sort keys are never rewritten: a kept link keeps
its sort key, and `drop_generated` keeps a link with a sort key, because the generated link has
none, and notes it. A link to the category page (`[[:Category:…]]`), interwiki links, and every
other link are not category links and are never touched. A link whose name holds `#`, `%`, or
`&` cannot be read from its text, since MediaWiki decodes those forms before it files the page
(`[[Category:Foo%20bar]]` files it in `Foo bar`); a page with one is refused whenever the plan
edits categories, and a plan may not name such a category. New lines use the page's usual line
break, CRLF when most of its line breaks are and LF otherwise, and the page keeps its final line
break, or its lack of one. New text never goes inside a node, a comment or literal region, or a
tag region such as a `ref`; a page where it would is refused.

**Generated categories.** The categories a class statement files a page in are read from
`Module:Ontology/display`, exactly as `Module:Infobox`'s `instanceOf` reads them: for each
class up to the first empty slot, the `classes[id].category` of the class, or of its nearest
superclass that has one. Superclasses are read through `subClassOf` as `Module:Ontology`'s
`find` reads it, from `classes`, `properties`, and `relations`, with no renames followed, for at
most fifteen classes; a `subClassOf` that lists several classes ends the walk. A category
marked `small` also files the page in the nearest superclass category that is not small. When
a class files the page in no category, the report notes why. A page that gets no class
statement because its typed infobox states the class has no generated categories, and
`drop_generated` removes nothing there. Nor does a translation subpage, one whose last subpage
is `es`, `ko`, `ja`, `zh`, `fr`, `de`, `pt`, or `ru` (`Wojak/ja`), since `Module:Infobox` files
those in no category. The report notes when `drop_generated` has nothing to do.

## Refusals

A refused page is not changed and has no edits. Every refusal is reported:

| Code | When |
|---|---|
| `redirect` | The page is a redirect. |
| `parse_failed` | The page's wikitext does not parse, for example an unclosed template. |
| `category_not_editable` | A link to a category the plan removes, adds, or drops sits inside a template, parameter, or link (`context` `template`, `parameter`, `link`), an `includeonly`, `noinclude`, `onlyinclude`, `gallery`, `ref`, `math`, `score`, or `templatedata` region, or a comment, `nowiki`, `pre`, `source`, or `syntaxhighlight` region; a link names its category with markup while the plan edits categories (`context` `unreadable`); or a new link would land in such a place. Links in comments and literal regions are found from their text, so a comment that does not parse cannot hide one. |
| `class_statement_not_editable` | The class-template call sits in such a place, the page has more than one (`context` `multiple`), its numbered slots are out of order or mixed with unnumbered ones, or a new call would land in such a place. |
| `ontology.unknown_class`, `ontology.renamed_term` | A planned class id is not a class, or was renamed, with the codes of `article lint`'s typing checks. It is checked on every page, with or without a class statement. The message names the current id of a renamed one. |
| other `ontology.*` codes | The result has a typing finding the page does not have, such as `ontology.unknown_field` for a planned field that `Module:Ontology/display` does not define as a relation field, which on the wiki files the page in a tracking category. Findings the page already has do not block. |
| `removal_missing` | The plan removes a category the page has no link to. |
| `invalid_plan` | The item contradicts itself or holds markup: a category both removed and added, an added category that `drop_generated` would remove, fields or `wikidata` without classes on a page with no call, a `wikidata` value that is not an item id, or a class id, field value, or category name holding `|`, braces, brackets, `<`, `>`, `__`, `~~~`, a control character, or a line break. |
| `page_missing` | The page has neither a draft nor a mirror file, or PATH names a file that is not the page's file. |
| `not_in_plan` | `--titles-file` or PATH names a title the plan does not list. |
| `not_an_article` | The title is in a namespace other than the article namespace. |
| `invalid_title` | The title holds a character no article title can hold, such as `\` or `#`. |
| `ambiguous_draft` | The title has more than one draft. |
| `draft_exists` | PATH names a mirror page whose title has a draft, or a file already exists at the draft path a mirror page would be written to. |
| `read_failed` | The page's file cannot be read as UTF-8 text. |
| `write_failed` | The draft cannot be written; the page's result is not reported as changed. |
| `duplicate_target` | Two selected titles would write one draft, or another file differs from the draft's name only in case. |
| `result_check_failed` | The result fails the check every result passes before it is reported: it parses, every node and tag region outside the edits is unchanged, each edit holds only one class call or one category link, and the category links change only as planned. It is a defect if it appears. |
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
| `draft_path`, `draft_written` | Where `--apply draft` writes the result, and whether it did; `null` when no file was read |
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
