---
name: contextmink
description: Use the project-local Contextmink CLI before generic repository reads, searches, structured-data inspection, or command capture may produce uncertain or high output. Skip known-small direct reads and project-native compact or domain-query commands.
---

# Use Contextmink

Use Contextmink as a low-friction output boundary. Reach for it before a generic
read can flood or silently clip the transcript; return to direct tools once the
exact path and a genuinely small result are known.

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

If `tools/contextmink/agent_integration.md` exists, read it completely before
changing setup, policy, hook, bridge, or capture behavior, or when receipt
semantics are material to a conclusion. Routine bounded reads can proceed from
this skill and live `--help`.

## Retrieve progressively

Start with the cheapest shape that answers the current question:

```text
contextmink dirs [PATH] --depth 2
contextmink files [PATH] --path-contains TEXT --ext rs --limit 20
contextmink grep --pattern PATTERN [PATH] --context 2 --limit 8
contextmink outline FILE --contains TEXT
contextmink slice FILE --range START:END
```

Use `grep-terms --term TERM [--term TERM] [PATH]` for shell-fragile literal
terms; terms are flags and paths are positional. Prefer `outline` then one
narrow `slice` over guessed dump windows.

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
