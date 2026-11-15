# Exact source-wikitext parsing

Wikitool's public source workspace includes `mediawiki_wikitext`, a small library for consuming
revision-bound MediaWiki source text. It complements `mediawiki_html_to_wikitext`: exact wikitext is
the preferred semantic input when a source wiki exposes it, while retained HTML remains a distinct
fallback for sources that do not.

The source-wikitext boundary accepts:

- a strict `mediawiki.wikitext-source-profile.v1` profile containing a source key, maximum byte and
  nesting bounds, source-reported file/category namespace aliases, and redirect magic words;
- a strict `mediawiki.wikitext-source-document.v1` receipt containing page, namespace, revision,
  timestamp, and content-model identity plus exact UTF-8 byte length and SHA-256 digest;
- the source bytes supplied by the caller.

The parser verifies the receipt before it exposes preorder, byte-span-addressed syntax nodes. It
balances template invocations, triple-brace parameters, and internal links; separates named and
positional template arguments; retains all file-link components; classifies links through the
source profile; exposes page-qualified fragments; and identifies leading redirects. Comments and
literal `nowiki`, `pre`, `source`, and `syntaxhighlight` regions are opaque protected nodes. Parsing
is bounded and fails locally on unclosed open constructs, malformed protected tags, or over-depth
nesting. Closing markers without an open construct remain literal source text, matching ordinary
code and prose behavior. Link recognition also leaves raw bracket expressions literal when their
would-be unexpanded title contains `[` or a lone `]`; syntactically possible unclosed links still
fail locally.

A `RewritePlan` names parsed nodes and their complete replacements. Replacements must not overlap,
which forces a consuming projector to choose whether an outer template or one of its descendants
owns a rewrite. With no replacements, output is byte-identical to the supplied source. The library
does not expand source templates or silently assign target semantics.

A plan can also remove a node with its line and insert text between nodes. A line removal takes
the whole line and its line break when nothing but spaces, tabs, and other removed nodes is on
it; a last line gives up the break before it instead. Otherwise it takes the node alone, with the
spaces after it when the node starts the line. An insertion must fall at a byte offset no node
spans across; several at one offset keep the order they were added in. `Document::resolve`
returns every edit's exact source span and replacement in source order, and rejects any overlap,
so a caller can report and verify precisely which bytes changed.

Each protected node also reports its unparsed interior, so a caller can inspect what a comment or
literal region holds without the parser reading it as wikitext. `Document::tag_regions` reports
`includeonly`, `noinclude`, `onlyinclude`, `gallery`, `ref`, `math`, `score`, and
`templatedata` regions found outside protected regions. They are not nodes, and their contents
are parsed like the surrounding text. A region runs from an opening tag to the next closing tag
of the same name, or to the end of the source when unclosed, so a tag does not nest in itself,
but a region of one tag can hold regions of the other tags, such as a `ref` inside `noinclude`.
As in MediaWiki's preprocessor, an opening tag ends at the first `>` after its name, whatever
quotes its attributes hold, and a closing tag allows only spaces between its name and `>`.

Corpus producers remain adapters. They enumerate and refresh pages, store bytes and media, and
produce their own manifests. A caller translates one verified page record into the neutral public
document receipt; Wikitool does not import or run the producer. A target-specific projector owns
template mappings, link/media routing, target capability checks, and a digest-bound transformation
receipt joining producer evidence, source profile, mapping profile, and output.
