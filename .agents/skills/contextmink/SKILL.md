---
name: contextmink
description: Use the project-local Contextmink CLI before generic repository reads, searches, structured-data inspection, or command capture may produce uncertain or high output. Skip known-small direct reads and project-native compact or domain-query commands.
---

# Use Contextmink

Use Contextmink as a low-friction output boundary. Reach for it before a generic
read can flood or silently clip the transcript; return to direct tools once the
exact path and a genuinely small result are known.

Choose it from the expected output, without waiting for the user to name it.
Routine retrieval needs no narration. Report evidence limitations when they
affect a conclusion, not every receipt or tool invocation.

## Resolve the project entrypoint

Read the repository's always-loaded guidance for its canonical command form.
Prefer the native Contextmink binary for the active host shell. In a
release-managed project it lives under
`tools/contextmink/bin/contextmink[.exe]`; Bash-hosted projects commonly expose
`scripts/contextmink`. In the Contextmink source checkout, follow its
`AGENTS.md`.

Do not cross a shell boundary for built-in Contextmink commands. Use
`contextmink-bridge` only for an intentional Bash script, Git Bash PATH tool, or
PowerShell-fragile argv workflow named by project guidance.

Routine retrieval and capture use this skill and live `--help`. Read
`tools/contextmink/agent_integration.md` completely before changing setup,
policy, hook, bridge, or capture integration. For an unfamiliar receipt field,
load its relevant reference section; ordinary interpretation is covered below.

## Retrieve progressively

Start with the cheapest shape that answers the current question. Search a known
subtree or file directly; directory orientation is optional. `dirs --depth`
bounds displayed levels, not traversal cost, so prefer narrow roots over an
inventory of an artifact store or dependency tree.

```text
contextmink dirs [PATH] --depth 2
contextmink files [PATH] --path-contains TEXT --ext rs --limit 20
contextmink grep --pattern PATTERN [PATH] --context 2 --limit 8
contextmink grep --pattern PATTERN FILE --lines-per-file 12 --max-sample-lines 24
contextmink outline FILE --contains TEXT
contextmink slice FILE --range START:END
```

Use `grep-terms --term TERM [--term TERM] [PATH]` for shell-fragile literal
terms; terms are flags and paths are positional. Prefer `outline` then one
narrow `slice` over guessed dump windows.

For grep, `--limit` counts matching files; `--lines-per-file` counts matches
within each file, and `--max-sample-lines` bounds all displayed matches and
context. `output_cap_arguments` names the exhausted display controls. Narrow
the query first. Raising the file limit cannot recover omitted per-file lines.
For a capped slice, `remaining_range` identifies the omitted part of the
requested window; pass it to `slice FILE --range ...` without raising caps.
These are live reads, not a snapshot: recheck relevant regions after edits.

For structured or command output, use the owned projection instead of opening
the full artifact:

```text
contextmink json-select FILE --keys
contextmink json-select FILE --fields FIELD[,FIELD] --limit 20
contextmink sqlite-schema DB --max-tables 20
contextmink sqlite DB --sql-file QUERY.sql --limit 20
contextmink capture --max-lines 40 -- PROGRAM ARGS
```

Prefer a domain tool's compact, projection, or limit flags over wrapping it in
`capture`. Use `capture` only when the child has no trustworthy native bound.

JSON fields are literal keys or JSON Pointers (`/result/total`); use `--at`
to enter nested objects or arrays. A displayed `<array:N items>` or
`<object:N keys>` is a summary, not its contents. Narrow with `--at` before
requesting those contents. Follow a producer's own pagination when available.

## Interpret evidence honestly

Read the final `contextmink.receipt.v2` fields, not just the visible rows:

- `scope_complete: false` means the inspected evidence was only a bounded
  subset.
- `output_truncated: true` means inspected payload was omitted or shortened.
- `result.total_is_lower_bound: true` forbids treating the total as exact.
- A subset no-match or all-null projection needs a narrower or corrected query.
- For capture, verify `child_exit_code`, `child_exit_zero`, and
  `exit_expected`.

Use `--fail-if-truncated` when every displayed result is required and
`--require-complete-scope` when bounded inspection cannot support the
conclusion. Narrow first; raise caps only with a concrete evidence need.

Keep direct known-small commands direct: `git status --short`, a focused test,
one exact small file region, or a project-native compact record does not benefit
from an extra Contextmink layer.
