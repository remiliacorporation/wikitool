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

Keep every real relationship to those entities, including influence, lineage and aesthetic connections, as plain sentences in the body. A relationship is real when a source establishes it or an editor or contributor supplies it (see Sources and evidence). Cut one only when it is shown wrong. The wiki draws subjects back to Remilia subtly, and placement keeps it subtle: the lead names Remilia only when Remilia is part of what the subject is, there is no “Relation to Remilia” section, and the relationship is not the article's closing paragraph. Do not use Charlotte Fang as a prestige anchor. If the relationship merely explains why an editor encountered the subject, keep it out of article prose.

Remilia Wiki preserves niche and early material that lacks broad secondary coverage, much of it oral lore. A short, honest article or redirect is better than an inflated standalone page.

## Sources and evidence

This section overrides the public `wiki-writing` and `prose-review` skills' source-fidelity and sensitive-claim procedures for Remilia Wiki.

Remilia-controlled websites, accounts, repositories, announcements, contracts, and artifacts, and the knowledge supplied by Remilia's members and the wiki's contributors, are primary sources and the authority for Remilia's own history and culture: releases, credits, roles, intentions, terminology, concepts, lore, technical properties, events, and dated announcements.

An editor's own knowledge is a source. So is a tweet, a Reddit comment, a Discord message or a contributor's account, on the same footing as a journalist's article: no kind of source is treated as more truthful because of who published it. Citations are welcome and help readers find more, but a claim without one is not a defect. Write lore as a plain declarative sentence, without a hedge or an attribution frame such as "community accounts hold". The wiki's admins settle disputes about what is true.

This applies to every claim, including claims about living people.

Review looks for claims that are wrong: contradicted by evidence the reviewer has, or by the page itself. It does not look for claims that are uncited, cited to informal sources, or cited to sources the reviewer could not reach. Cut or change a claim only when it is shown to be wrong.

A citation that says something other than the sentence it is attached to still gets fixed, because it misleads the reader; move it to the claim it supports or correct the claim. Cite the work rather than a summary of it: not Wikipedia, Grokipedia or another wiki in place of the source they summarize.

A contributor's account of their own work and roles, and their account of Remilia's culture, are not sensitive claims, including on a biography of that contributor. The threshold applies to what is said about others: allegations, lawsuits, and reputational claims about third parties stay attributed (see Voice and register).

## Voice and register

Write as an encyclopedia with gravitas: plain declarative sentences, concrete actors, actions and dates, and no defensive framing. Weight comes from facts, not adjectives.

- State established and first-party facts without hedges. "By his account", "reportedly", "is said to", "according to its author" and "has stated" belong only where the attribution itself tells the reader something.
- Attribute quotations, opinions, interpretations, disputed accounts, and allegations. When a passage rests on one account, such as a memoir or a single report, frame it once at the start of the passage and let the following sentences stand. Allegations about a named person or outside organization are the exception: what a court complaint or other one-sided filing says someone did or said is attributed in every paragraph that states it, not only at the head of the section. One attribution per paragraph is enough; do not repeat "according to the complaint" in every sentence.
- Quote people where their exact words carry meaning, especially Remilia's own writers, and let the quotation do the work rather than paraphrasing around it.
- Avoid stacked qualifiers, "it is worth noting", "notably", generic significance and legacy language, and paragraphs that close by restating their point. Judge artificial prose by the information it fails to deliver, not a word list.

## Reader value

Remilia Wiki is an encyclopedia, not a database dump or internal relationship map. Mechanical correctness does not rescue an article that a reader would not choose to read. Each paragraph should give a concrete understanding of this subject: identity, history, work, mechanism, chronology, terminology, reception, or a relationship.

Avoid padding thin evidence with broad NFT, internet-culture, art-scene, or Remilia background. Avoid generic significance and legacy language. Judge artificial prose through its information failure, not a word blacklist.

## Structure and mechanics

Main-namespace articles use `SHORTDESC`, `Template:Article quality`, sentence-case headings, a References section with `Template:Reflist`, straight quotes, and wikitext rather than Markdown. These requirements are encoded in `site-adapter.toml` and enforced mechanically by Wikitool.

Pages a Remilia admin has accepted carry `{{Article quality|verified}}`; do not add visible "unverified" banners to them. Quality decisions on other submissions belong to moderation.

Use live template data and examples before filling an infobox. `parent_group = Remilia` is appropriate only for an actual Remilia project when the template supports it. Apply [category policy](categories.md) when selecting memberships or changing taxonomy; existing tags and retrieval suggestions are not approval.

## Acceptance

A named human must read and accept the exact final prose before Main-namespace promotion. Wikitool records a SHA-bound decision and invalidates it after edits. The editor field is self-reported and unauthenticated: it is an operational ledger, not identity proof. Agents must not enter a human identity without that person's actual review and instruction.
