# Remilia Wiki research playbook

This document supplements [editorial policy](editorial.md). It says where to look before drafting an article, how much to search, and what to record so that a reviewer can see where every statement came from.

## Two kinds of subject

How much research a subject needs depends on where its record lives.

- **A subject with an outside record** is a meme, site, community, person or term that existed outside Remilia. Its record is spread across public archives, and research is how the article finds it.
- **A first-party or contributor-supplied subject** is Remilia's own work, history or culture, or any subject whose record is mainly what the contributor supplies. The contributor's material leads, and research corroborates it.

When one article has parts of both kinds, treat each part by where its record lives.

## Where to look

Search in this order. Move past a source when it has nothing more for the subject.

1. Know Your Meme: the entry, its sources, and its dates.
2. Wikipedia: the article for the subject and for near subjects, then its references.
3. Wayback Machine and archive.today: dead sites, deleted pages, and the original form of a page.
4. 4chan archives (desuarchive, archived.moe): the first threads, with dates and board.
5. X search, Reddit, Tumblr, and Substack: first posts, early discussion, and participant accounts.
6. YouTube and podcast transcripts: interviews, documentaries, and video essays.
7. News search, Google Books, GitHub, and on-chain records: reporting, books, code history, and contract data.

For a first-party subject, start with the contributor's material, then Remilia's own sites, accounts, repositories and contracts, then the list above where it applies.

This is the order to search, not the order to cite. The early entries are where the artifact is usually found; [What to cite](editorial.md#what-to-cite) puts the artifact first.

Not every source applies to every subject. A subject from 2008 may have nothing on Substack. A token has on-chain records and a joke has none.

## How much to search

### Subjects with an outside record

Before drafting, run at least eight distinct queries across at least four of the sources above. A query that only changes a stop word does not count as distinct. Vary the name, the aliases, the date, the platform, and the people involved.

The minimum is a floor against thin research, not a quota. Do not fill it with near-duplicate queries. Past the floor, stop when new queries return only what you already have.

If research and the contributor's material together yield too little for a full article, write a short article or a redirect, and record in the notes what was searched.

### First-party and contributor-supplied subjects

- Draft from the contributor's material. Research corroborates it and does not gate it.
- Search for the public artifacts that plausibly exist, such as a post, credit, release, commit, contract, log or announcement, and cite them where they help the reader.
- Scale the search to what plausibly exists. An NFT release has a contract and usually an announcement; an in-joke from a group chat may have no public artifact at all.
- No fixed minimum applies. Log the searches you ran, including the empty ones.
- An empty search is never a reason to withhold, weaken or attribute a supplied statement ([Contributors](editorial.md#contributors)).
- When a source contradicts the contributor's material, ask the contributor before writing ([Contributors](editorial.md#contributors)).

## Notes

Keep notes for every draft, for example `.wikitool/drafts/TITLE.notes.md`. They hold the search log and a provenance line for every statement that carries no citation. Together they answer "where did this come from" for each statement in the draft, as [Provenance](editorial.md#provenance) requires.

### Search log

One line per query:

```
source | query | result (hits, or none) | used
```

Include the empty searches. The log lets a reviewer see what was tried and lets the next editor skip it. Record each fetched page with its URL and access date, and each failed archive attempt (see [Archive what you cite](#archive-what-you-cite)).

### Provenance lines

One line per uncited statement, or per group of statements from the same place:

```
statement (short form) | origin | locator | date received or read
```

- `origin` is `contributor`, or the URL of a document opened and read for this article.
- `locator` says where in that material the statement is: a draft paragraph, an answer to a question, a message, or a passage. When an interview ledger exists, point to its entry instead of copying the answer.

For example: `network spirituality: definition | contributor | draft, paragraph 2 | 2026-11-12`.

## Archive what you cite

Archive every X or social post you cite before you cite it. Use the Wayback Machine "Save Page Now" function or archive.today. Put the archive URL and date in the citation (`archive-url` and `archive-date`). Do the same for any page that is likely to change or vanish.

Fill archive fields only with an archive URL and date that you opened and checked. Never fill them with an unverified URL.

An archive service can refuse or fail; X posts often do. Then:

- Try the other service.
- Record each failed attempt in the search log: service, URL, date, and the error.
- Cite the live URL.
- Leave `archive-url` and `archive-date` blank.

For stable sources such as books and published articles, the archive fields may stay blank.

## From search to sentences

- Follow every reference site (Know Your Meme, Wikipedia, a Fandom wiki, Urban Dictionary) to the work it summarizes, read the work, and cite that ([What to cite](editorial.md#what-to-cite)).
- If the work cannot be opened, never cite it on the strength of the summary's description. Record the attempt in the search log. [What to cite](editorial.md#what-to-cite) decides whether the summary itself may be cited.
- Source type does not change how firmly a sentence speaks. A statement from a tweet, a contributor or a newspaper is written in the same plain voice; attribution is only for what [Voice and register](editorial.md#voice-and-register) lists.
- Do not take facts from model memory. A recalled fact is a lead for a search or a question for the contributor, never a sentence ([Provenance](editorial.md#provenance)).
- Do not go looking for a tie to Remilia. Record one only when a source states it or the contributor supplies it ([Article object and framing](editorial.md#article-object-and-framing)).
