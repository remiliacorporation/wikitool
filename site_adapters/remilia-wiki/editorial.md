# Remilia Wiki editorial supplement

This document specializes the public Wikitool authoring and review skills for Remilia Wiki. It is project policy, not generic MediaWiki behavior and not executable prose policy.

## A first-party record

Remilia Wiki is Remilia's own encyclopedia of its culture and history, written in large part by the people who made it and lived it. Like a game studio's or a fan community's wiki, it is a first-party record, and one of its purposes is to canonicalize: much of Remilia's and its adjacent subcultures' knowledge (terminology, concepts, lore, in-jokes, aesthetics, chronology, who made what) exists nowhere else in citable form, and the wiki is where it becomes the reference.

Wikitool is public, and anyone may use it to contribute. The person using it is a contributor: possibly a Remilia member, an artist, a longtime community member or a newcomer. Do not assume who they are, whether they hold a role at Remilia or on the wiki, or that they are the subject of the article; do not infer identity from a name, handle or topic. If their relation to a subject matters to the article, ask.

The contributor is a source to work with, not a witness to be qualified. What they write or supply as fact goes into the wiki's own voice:

- Their own work, credits, roles, decisions and intentions. Write "She designed the guestbook", not "by her account, she designed the guestbook".
- Their knowledge of Remilia and its adjacent subcultures: what a term or concept means, how a meme or practice originated, what a work or event was, who made what. When the contributor says "network spirituality means …", the article says "Network spirituality is …", not "according to the contributor, network spirituality is …".
- Their drafts and suggested wording. Keep it, and build the article around it.

Corroborate with citations where a public artifact exists (a post, credit, release, commit, log, scan, essay or announcement) and where it helps the reader. Never withhold, weaken or attribute a statement merely because no outside source repeats it; the absence of an outside source is the reason the wiki exists, not a defect in the claim.

Question a supplied statement only when there is a real reason: it is implausible on its face ("dogs are fish"), it contradicts an inspected source or another supplied statement, or it makes an allegation about an identifiable person or outside organization (misconduct, legal or reputational claims). Then ask the contributor for the basis before writing, and write what is resolved. Do not convert doubt into hedged prose. Promotion is the other limit: state what was made, done or meant, and leave praise and claims of outside importance to sources that establish them.

Write neither direction by default. Do not weaken a contribution to protect the wiki, and do not strengthen it beyond what the contributor said. Submissions pass through the wiki's moderation queue, and Remilia's admins adjust voice, scope and confidence there; the agent's job is an accurate, plain rendering of what the contributor supplied.

Review applies the same rule. A reviewer reports contributed content only for those reasons or for a misused citation, never for lacking attribution or a hedge.

## Article object and framing

Define every subject on its own terms. A page's presence on Remilia Wiki does not make Remilia, Charlotte Fang, Milady Maker, or “the community” part of the subject's definition.

Include a relationship to those entities only when all three conditions hold:

1. an inspected source establishes the relationship;
2. the relationship materially helps a reader understand the subject;
3. its placement and length are proportionate to its importance in the source record.

Do not create a default “Relation to Remilia” section. Do not open a lead with local relevance or use Charlotte Fang as a prestige anchor. If the relationship merely explains why an editor encountered the subject, keep it out of article prose.

Remilia Wiki can preserve niche and early material that lacks broad secondary coverage. That makes source integrity and bounded primary-source description more important, not less. A short, honest article or redirect is better than an inflated standalone page.

## Sources

Remilia-controlled websites, accounts, repositories, announcements, contracts, and artifacts, and the knowledge supplied by Remilia's members and the wiki's contributors, are primary sources and the authority for Remilia's own history and culture: releases, credits, roles, intentions, terminology, concepts, lore, technical properties, events, and dated announcements. They do not independently establish outside importance, reception, consensus, or claims about people and organizations outside Remilia.

Remilia Wiki pages are retrieval aids and claim trails. Inspect their underlying citations before reusing a claim. Do not cite the wiki as independent confirmation unless the wiki page itself is the primary artifact being discussed.

Wikipedia is a tertiary discovery surface. Follow its references and cite the underlying work. Never compress a Wikipedia article's many per-claim citations into a single Wikipedia citation or a generic publisher home page. A direct Wikipedia citation needs an explicit article-specific reason, such as discussing the Wikipedia page itself.

The adapter's source-review matches are reread signals, not automatic bans. Explain why a matched source is or is not suitable for the particular claim.

## Living people and sensitive claims

Apply the public `prose-review` sensitive-claim procedure to any living or plausibly living person. Drug use, health, crime, arrests, allegations, sexuality, identity, private relationships, harassment, finances, and reputational controversies require direct high-quality support, exact attribution, and a due-weight reason for inclusion.

Do not let local familiarity, a social-media thread, a tertiary biography, or repeated community lore lower the threshold. If the source packet cannot support the claim, omit it and preserve it as a hold outside article prose.

A contributor's account of their own work and roles, and their account of Remilia's culture, are not sensitive claims, including on a biography of that contributor. The threshold applies to what is said about others: allegations, lawsuits, and reputational claims about third parties stay attributed and sourced.

## Voice and register

Write as an encyclopedia with gravitas: plain declarative sentences, concrete actors, actions and dates, and no defensive framing. Weight comes from facts, not adjectives.

- State established and first-party facts without hedges. "By his account", "reportedly", "is said to", "according to its author" and "has stated" belong only where the attribution itself tells the reader something.
- Attribute quotations, opinions, interpretations, disputed accounts, and allegations. When a passage rests on one account, such as a court complaint, frame it once at the start of the passage or section and let the following sentences stand; do not repeat "according to the complaint" in every sentence.
- Quote people where their exact words carry meaning, especially Remilia's own writers, and let the quotation do the work rather than paraphrasing around it.
- Avoid stacked qualifiers, "it is worth noting", "notably", generic significance and legacy language, and paragraphs that close by restating their point. Judge artificial prose by the information it fails to deliver, not a word list.

## Reader value

Remilia Wiki is an encyclopedia, not a database dump or internal relationship map. Mechanical correctness does not rescue an article that a reader would not choose to read. Each paragraph should give a concrete understanding of this subject: identity, history, work, mechanism, chronology, terminology, reception, or a sourced relationship.

Avoid padding thin evidence with broad NFT, internet-culture, art-scene, or Remilia background. Avoid generic significance and legacy language. Judge artificial prose through its information failure, not a word blacklist.

## Structure and mechanics

Main-namespace articles use `SHORTDESC`, `Template:Article quality`, sentence-case headings, a References section with `Template:Reflist`, straight quotes, and wikitext rather than Markdown. These requirements are encoded in `site-adapter.toml` and enforced mechanically by Wikitool.

Pages a Remilia admin has accepted carry `{{Article quality|verified}}`; do not add visible "unverified" banners to them. Quality decisions on other submissions belong to moderation.

Use live template data and examples before filling an infobox. `parent_group = Remilia` is appropriate only for an actual Remilia project when the template supports it. Apply [category policy](categories.md) when selecting memberships or changing taxonomy; existing tags and retrieval suggestions are not approval.

## Acceptance

A named human must read and accept the exact final prose before Main-namespace promotion. Wikitool records a SHA-bound decision and invalidates it after edits. The editor field is self-reported and unauthenticated: it is an operational ledger, not identity proof. Agents must not enter a human identity without that person's actual review and instruction.
