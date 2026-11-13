# Remilia Wiki research playbook

This document supplements [editorial policy](editorial.md). It says where to look before drafting a new article about an internet-culture subject, and what to record.

## Where to look

Search in this order. Stop early on a source only when it has nothing.

1. Know Your Meme: the entry, its sources, and its dates. It is a lead, not a final source.
2. Wikipedia: read the article for the subject and for near subjects, then follow its references to the underlying works.
3. Wayback Machine and archive.today: dead sites, deleted pages, and the original form of a page.
4. 4chan archives (desuarchive, archived.moe): the first threads, with dates and board.
5. X search, Reddit, Tumblr, and Substack: first posts, early discussion, and participant accounts.
6. YouTube and podcast transcripts: interviews, documentaries, and video essays.
7. News search, Google Books, GitHub, and on-chain records: reporting, books, code history, and contract data.

Not every source applies to every subject. A subject from 2008 may have nothing on Substack. A token has on-chain records and a joke has none.

## Minimum effort

Before drafting a new article, run at least eight distinct queries across at least four of the sources above. A query that only changes a stop word does not count as distinct. Vary the name, the aliases, the date, the platform, and the people involved.

If the minimum yields too little to write a sourced article, write a short article, or a redirect, and say what was searched.

## Search log

Keep a search log in the draft's notes, for example `.wikitool/drafts/TITLE.notes.md`. One line per query:

```
source | query | result (hits, or none) | used
```

Include the empty searches. The log lets a reviewer see what was tried and lets the next editor skip it. Record fetched pages with their URLs and access dates.

## Archive what you cite

Archive every X or social post you cite before you cite it. Use the Wayback Machine "save" function or archive.today. Put the archive URL in the citation (`archive-url` and `archive-date`). Do the same for any page that is likely to change or vanish.

Fill archive fields only with an archive URL and date that you opened and checked. For stable sources such as books and published articles, the fields may stay blank.

## From search to sentences

- Follow every tertiary page (Know Your Meme, Wikipedia, a Fandom wiki) to the work it cites. Cite the work if you can read it. If you cannot, use the tertiary page only for a date or a lead, and say less.
- Note the rung of each source on the [source ladder](editorial.md#source-ladder) while you read. It decides how firmly a sentence can speak.
- Keep lore apart from sourced claims in your notes, so that it is clear which sentences have a citation and which are plain lore.
- Do not take facts from model memory. If you recall a fact, find where it is written.
