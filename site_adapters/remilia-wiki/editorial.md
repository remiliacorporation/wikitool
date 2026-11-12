# Remilia Wiki editorial supplement

This document specializes the public Wikitool authoring and review skills for Remilia Wiki. It is project policy, not generic MediaWiki behavior and not executable prose policy.

## Mission

Remilia Wiki is a general internet-culture wiki. It is not a Remilia promotion site. Its citation bar is well below Wikipedia's, because much of internet culture is oral lore and other material that no article has covered. Write each subject on its own terms, as fully as the evidence allows.

Companion documents: [research](research.md) for where to look, [style](style.md) for voice and structure, and [review](review.md) for the coverage step of prose review.

## Article object and framing

Define every subject on its own terms. A page's presence on Remilia Wiki does not make Remilia, Charlotte Fang, Milady Maker, or "the community" part of the subject's definition.

In an article about a non-Remilia subject, apply due weight in hard terms:

1. Mention Remilia, Milady Maker, Charlotte Fang, or Miya only when a source outside Remilia's own channels makes the connection, or when the connection defines the subject (for example, Remilia made it).
2. Put the mention in the body. Never put it in the lead, in a dedicated section, or in the closing paragraph.
3. Never add `Category:Remilia` to a non-Remilia subject.
4. Do not write a "Relation to Remilia" section, and do not use "adjacent" to link the subject to Remilia.

If the connection merely explains why an editor encountered the subject, keep it out of article prose. Do not open a lead with local relevance or use Charlotte Fang as a prestige anchor. Articles about Remilia's own work are exempt from this section.

Remilia Wiki can preserve niche and early material that lacks broad secondary coverage. That makes source integrity and bounded primary-source description more important, not less. A short, honest article or redirect is better than an inflated standalone page.

## Sources

Remilia-controlled websites, accounts, repositories, announcements, contracts, and artifacts are primary sources for their own content. They may establish releases, stated intentions, credited roles, technical properties, and dated announcements. They do not independently establish broader importance, reception, consensus, or the truth of self-serving claims.

Remilia's own channels are primary sources for Remilia's own acts. They are never the main source for an article about another subject.

### Source ladder

Rank sources from strongest to weakest. Use the strongest rung that supports the claim, and say less when only a weak rung does.

1. Secondary reporting: news, books, academic work, and established blogs and podcasts.
2. Archived primary posts and threads: the post or thread itself, saved in an archive.
3. Reference sites: Know Your Meme, Fandom wikis, and Urban Dictionary. Use them for dates and leads, then follow their sources.
4. Recorded participant testimony: interviews and notes from people who were there.
5. Lore.

The bar falls with the rung. A consequential claim needs rung 1 or 2. A date or a name can rest on rung 3.

### Lore

Lore is knowledge held by the community that no article records: how a meme began, what a phrase meant, who did what in a thread. Write lore as a plain declarative sentence, with a citation if one exists and without one if none does.

Never use an attribution frame. Do not write "community accounts hold that", "is said to", "according to users", or "is often claimed". The frame hedges a statement the article is willing to make.

Lore comes from the editor's notes, an interview, or archived posts. It never comes from model memory. If two accounts of the lore conflict, state the dispute once, with who disputes it.

The living-people limit below still applies. Lore never lowers the threshold for a sensitive claim about a living person.

Remilia Wiki pages are retrieval aids and claim trails. Inspect their underlying citations before reusing a claim. Do not cite the wiki as independent confirmation unless the wiki page itself is the primary artifact being discussed.

Wikipedia is a tertiary discovery surface. Follow its references and cite the underlying work. Never compress a Wikipedia article's many per-claim citations into a single Wikipedia citation or a generic publisher home page. A direct Wikipedia citation needs an explicit article-specific reason, such as discussing the Wikipedia page itself.

The adapter's source-review matches are reread signals, not automatic bans. Explain why a matched source is or is not suitable for the particular claim.

## Living people and sensitive claims

Apply the public `prose-review` sensitive-claim procedure to any living or plausibly living person. Drug use, health, crime, arrests, allegations, sexuality, identity, private relationships, harassment, finances, and reputational controversies require direct high-quality support, exact attribution, and a due-weight reason for inclusion.

Do not let local familiarity, a social-media thread, a tertiary biography, or repeated community lore lower the threshold. If the source packet cannot support the claim, omit it and preserve it as a hold outside article prose.

## Reader value

Remilia Wiki is an encyclopedia, not a database dump or internal relationship map. Mechanical correctness does not rescue an article that a reader would not choose to read. Each paragraph should give a concrete understanding of this subject: identity, history, work, mechanism, chronology, terminology, reception, or a sourced relationship.

Avoid padding thin evidence with broad NFT, internet-culture, art-scene, or Remilia background. Avoid generic significance and legacy language. Judge artificial prose through its information failure. The [style guide](style.md) lists words to watch as signals to re-read a sentence, not as bans.

## Structure and mechanics

Main-namespace articles use `SHORTDESC`, `Template:Article quality`, sentence-case headings, a References section with `Template:Reflist`, straight quotes, and wikitext rather than Markdown. These requirements are encoded in `site-adapter.toml` and enforced mechanically by Wikitool.

Use live template data and examples before filling an infobox. `parent_group = Remilia` is appropriate only for an actual Remilia project when the template supports it. Apply [category policy](categories.md) when selecting memberships or changing taxonomy; existing tags and retrieval suggestions are not approval.

## Acceptance

A named human must read and accept the exact final prose before Main-namespace promotion. Wikitool records a SHA-bound decision and invalidates it after edits. The editor field is self-reported and unauthenticated: it is an operational ledger, not identity proof. Agents must not enter a human identity without that person's actual review and instruction.
