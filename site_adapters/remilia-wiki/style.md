# Remilia Wiki style guide

This document supplements [editorial policy](editorial.md). It sets the voice and structure of an article. The aim is an encyclopedic article that reads at least as well as Wikipedia's article on the same subject.

## Voice

- Name the subject first. The first sentence says what the subject is, in the subject's terms.
- Use plain past tense for history. Use present tense for what is still true.
- Prefer one fact per sentence. Split a sentence when its clauses need different sources.
- State a disputed point once, with who disputes it. Do not repeat the dispute in every paragraph.
- Write lore as plain declarative sentences. See [Lore](editorial.md#lore).
- Quote exactly, and only as much as carries the meaning ([Voice and register](editorial.md#voice-and-register)). Use a block quotation for a text the reader needs whole, such as a definition, a license or an essay passage. Quote a passing remark inline. Name the speaker once.
- Say what happened, who did it, and when. Leave the judgment of importance to the reader and the sources.

## Words to watch

These words are signals to re-read a sentence, not bans. Keep one only when the sentence would be false or unclear without it.

| Word | Why it is a problem |
|---|---|
| adjacent | Names no relation. Say what the relation is: who did, said or made what, and when. The relationship stays; the word goes. |
| lineage | Names a descent without its steps. Name the predecessor and what passed from it, as a concrete "X did Y". A real lineage stays ([Article object and framing](editorial.md#article-object-and-framing)). |
| ironic detachment | A label for a tone, not a fact. Show the behavior that you mean. |
| broader | Points at a larger context without naming it. Name the context. |
| notably | Asks the reader to find a fact important. Put the fact in the sentence. |
| significant | A judgment with no measure. Give the number, the date, or the source that says so. |
| pivotal | Claims a turning point. Say what changed before and after. |
| landscape | Abstract noun that stands in for a place or a scene. Name the scene. |
| ecosystem | Abstract noun that stands in for a set of projects or people. Name them. |
| ethos | Abstract noun for a set of values. State the values, or quote the people who held them. |

## Headings

Headings are specific to the subject and its type. Use sentence case. Do not use "Background", "Overview", "Legacy", or "Influence" as a default heading. Use one only when the section under it holds concrete facts about the subject; under "Influence", that means who took what from the subject, or what the subject took from whom. As of the 2026-11-12 review, "Background" appeared on 187 pages, and on most of them it said nothing about the subject.

Start from the set for the subject's type, and drop any section the evidence cannot fill. A short article may have no sections at all.

| Type | Headings |
|---|---|
| Meme | Origin, Spread, Variants, Usage, Reception |
| Online community | History, Platforms, Culture, Notable events, Reception |
| Website | History, Features, Users, Reception |
| Person | Early life, Career, Works, Reception |
| Term | Origin, Meaning, Usage, Related terms |
| Concept | Origin, Definition, Applications, Criticism |
| Event | Background, Events, Aftermath |
| NFT collection | Creation, Mint, Traits, Market |
| Organization | History, Activities, Reception |
| Artwork | Creation, Description, Exhibition, Reception |

Event keeps "Background" because there it names the lead-up to a dated event. Drop it when there is nothing to lead up to.

## Mechanics

Wikitool's `article lint` enforces three of these rules: curly quotes (`style.curly_quotes`), footnote placement (`citation.after_punctuation`), and heading capitalization (`style.sentence_case_heading`, which catches title case in headings of three or more words, not every miscapitalization). Writers and reviewers check the rest.

These rules set spelling, punctuation, and form. They follow Wikipedia's Manual of Style, with the choices below made for this wiki. A rule that says "keep consistent" means the article may not mix the two forms.

- Use US spelling: color, behavior, organize, center. Quotations and proper names keep their own spelling.
- Write dates as month, day, year: May 12, 2019. Do not mix this with 12 May 2019 in one article.
- Use the serial comma: "Reddit, Tumblr, and Twitter".
- Use straight quotes and apostrophes. Do not use curly ones.
- Put a footnote after the punctuation it follows: `text.<ref>`, not `text<ref>.`.
- Use logical quotation. Put a comma or period inside the closing quote only when it belongs to the quoted words. Write `"Much hug", and "Wow".` for fragments.
- Use `'s` for the possessive of a singular name that ends in s: "James's book", "Descartes's method". Plurals take a bare apostrophe: "the Beatles' album".
- Use an en dash for a range: 2019–2021. Do not use a hyphen, and do not use `--` for a dash. Pick spaced or unspaced for the dash that sets off a phrase, and keep it the same through the article.
- Use three periods for an ellipsis, with no spaces between them.
- Put no space before a comma, period, colon, semicolon, or closing parenthesis.
- Write "and" or "or" instead of a slash between words: "artist and musician", not "artist/musician". Board names (/x/), subreddits (r/backrooms), and URLs keep their slashes.
- Write "and" in prose, not "&", except in a name.
- Use "×" for dimensions and "number" for "#" in prose: "1920×1080", "number one". Keep "#" in token IDs and color codes.
- Write in the third person. Do not use "I", "we", "you", or "your" outside a quotation.
- Write the words out: "do not", not "don't". Quotations keep their contractions.
- Give a year or a date, not "recently", "currently", "today", or "last year". An article is read years after it is written.
- Write headings in sentence case, with no leading "The", "A", or "An" unless it is part of a name. Do not end a heading in punctuation, and do not put links, refs, templates, or math in one.
- Do not repeat the article title in a heading.
- Bold the title, or a form of it, in the first sentence. Open with a direct "X is a ..." sentence, not "X refers to ...".
- Keep the lead to one or two paragraphs for most articles. A very long article may take three or four.
- Use one blank line between paragraphs. Do not leave two or more.
- Do not end a sentence with "?" or "!" outside a quotation.

## Worked examples

Both examples come from existing pages. The "After" text shows the form only: each fact in it still needs a citation or a [provenance line](research.md#provenance-lines) in the live article.

### The lead of 4chan

Before:

> '''4chan''' is an anonymous English-language imageboard website founded in 2003 by Christopher Poole, known online as "moot." Structured around topic-specific boards where users post images and text without registering an account, 4chan became one of the most influential forces in the development of internet culture, meme production, and online subculture through the 2000s and 2010s. It is the originating environment for a significant portion of the aesthetic and cultural vocabulary that circulates in communities adjacent to [[Remilia Corporation]], including anonymous posting culture, ironic detachment, and the pipeline from niche subcultural interest to mainstream meme diffusion.

After:

> '''4chan''' is an English-language imageboard where users post images and text without registering an account. Christopher Poole, known online as "moot", launched it on October 1, 2003, and modeled it on the Japanese imageboard Futaba Channel. Its topic-specific boards include /b/ (Random), /a/ (Anime and Manga), and /v/ (Video Games). Hiroyuki Nishimura, the founder of 2channel, bought the site in 2015 and has operated it since.

What changed:

- The first sentence says what the site is and how it works. "Influential" and "internet culture" are gone, because the history sections show the influence.
- The Remilia sentence left the lead. Remilia is not part of what 4chan is, so it does not belong there (placement rule 1 in [Article object and framing](editorial.md#article-object-and-framing)).
- "Adjacent" and "ironic detachment" name no relationship, so they went with it. If a source or a contributor supplies a concrete relationship, it goes in the body where its topic is discussed, stated as who did what.
- The dates, names, and boards that the old lead left for later are now in the lead.

### The closing paragraph of Pepe the Frog

Before (the last paragraph of "Interpretation and influence", with the page's `Category:Remilia`):

> Pepe is also a reference point for [[Remilia Corporation]] and the [[New Net Art]] movement. The Rare Pepe cards described above prefigured the profile-picture NFT projects that [[Milady Maker]] later developed, and [[Charlotte Fang]] has described New Net Art as art unafraid to engage the internet, in contrast with "artists [who] cowered in irrelevance scared to engage the internet because they got cancelled for knowing what pepe is." The Miya corpus, which preceded Remilia, uses egregore language for Pepe [...]

After, in the part of the body that discusses the Rare Pepe cards:

> The Rare Pepe cards prefigured later profile-picture NFT projects, including [[Milady Maker]].

"Interpretation and influence" now ends on the paragraph before the old one:

> Furie started a "Save Pepe" campaign, killed the character off in 2017, and took legal action over several uses. The image stayed in circulation.

`Category:Remilia` is removed.

What changed:

- The concrete relationship stays. That the Rare Pepe cards prefigured profile-picture NFT projects, Milady Maker among them, is now one plain sentence where the cards are discussed (placement rule 2).
- "A reference point for Remilia Corporation and the New Net Art movement" names no relationship, so the sentence does not stay as written. Ask the contributor what Remilia or New Net Art did with Pepe; the answer goes where its topic is discussed (placement rule 3).
- Charlotte Fang's quotation goes. It is about New Net Art rather than Pepe, and here it served as a prestige anchor (placement rule 5).
- The Miya corpus sentence, cut short in this example, follows the same rule: it stays as a plain sentence where the section discusses interpretations of Pepe, not as the closing paragraph.
- The old paragraph cited only a Remilia Wiki page. The live article cites what that page cites, or records provenance ([What to cite](editorial.md#what-to-cite)).
- `Category:Remilia` marks Remilia's own work, and Remilia did not make Pepe (placement rule 4).
- The section ends on its subject.
