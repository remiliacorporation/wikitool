# Contextmink project reference

This is the detailed, release-managed integration reference behind the
project-local Contextmink skill. Keep the skill generic; adapt repository-owned
shell, native-tool, nested-repository, exclusion, and destructive-path policy
in always-loaded guidance and `.contextmink.toml`.

## Bounded Output

Use contextmink when a file/text/JSON/SQLite/command-output read may produce
more output than the transcript should carry.

- Establish the repository's intended workspace root before using repo-local
  entrypoints. The relative command forms below assume that root; from a nested
  working directory, use an absolute/root-resolved launcher path or return to
  the workspace root first.
- Choose invocation by the active shell and target: use `scripts/contextmink ...`
  from Bash-hosted sessions such as macOS, Linux, Git Bash, or WSL;
  use `& tools\contextmink\bin\contextmink.exe ...` directly from Windows
  PowerShell for contextmink commands; use
  `& tools\contextmink\bin\contextmink-bridge.exe --script scripts/contextmink ...`
  when a PowerShell-hosted Windows session needs the Bash launcher.
- When the target file is unknown, start with `dirs` to orient in the tree,
  then use `files` or `grep` for candidate discovery. Narrow file discovery
  with repeated `--path-contains` values and `--ext` before raising display
  limits. Prefer
  `files --ext json` (or `--ext jsonl`)
  (comma-separated lists work: `--ext rs,toml`) across Windows-to-Bash
  boundaries because wildcard globs can expand before contextmink receives
  them.
- Once the file is known but the relevant region is not, use `outline` then
  `slice`, not dump windows. `outline <file>` maps declaration lines with line numbers
  (`--contains TEXT` filters rows; `--lang`, `--prefix <text>`, or
  `--pattern <regex>` cover unrecognized extensions), then
  `slice --range START:END` prints the region. Use `slice` when a file window
  may be broad or needs bounded follow-up; keep direct exact small reads direct.
  Keep its default caps (120-line window,
  220-line ceiling); narrow an oversized read with `outline` or
  `grep --context` instead of raising `--max-lines`.
  Built-in outline matching is a disclosed navigation heuristic over
  comment/string-masked text; use explicit prefix or regex matching when the
  desired anchor is not a declaration shape.
- Use `grep --pattern-file <file>` for shell-fragile regex; use `grep-terms`
  for literal tokens or phrases (AND by default; pass `--any` for OR). Load
  phrases with `--term-file` and cap with `--limit` /
  `--max-sample-lines`. Bound inspected content deterministically with
  `--max-content-files` or `--max-content-bytes`. Narrow
  either with `--glob` / `--ext`, add `-i` for
  case-insensitive matching, and `--context N` when the surrounding lines
  would otherwise need a follow-up `slice`.
- Use `slice --tail N` for the end of logs, `json-find`, `json-select` (with
  `--where FIELD=VALUE` / `--where-contains FIELD=TEXT` row filters;
  `--keys` first when the row shape is unknown), `sqlite-schema`, and
  `sqlite --sql-file` for bounded reads instead of opening whole large
  files, reports, or databases. JSON object keys must be unique, every
  non-empty physical JSONL line is one value, and `--max-document-bytes`
  bounds a materialized JSON document or one streamed JSONL record.
  `sqlite` binds JSON/JSONL worklists as
  named parameters (`--jsonl-param w=file.jsonl` with `json_each(:w)`) and
  registers `hexint(x)` for joining `0x...` hex strings against integer
  columns.
- Prefer a domain command's native compact/projection/limit flags first. Use
  `capture -- <command> ...` only when output size is uncertain and no
  native bound exists; read `child_exit_code`, `child_exit_zero`, and
  `exit_expected` in the receipt. Direct capture recognizes files whose first
  line begins `#!`; use
  `capture --script -- <script> ...` for a no-shebang Bash script. Truncated
  captures keep both the head and the tail of each separately bounded stream;
  they do not invent stdout/stderr chronology.
- Configured excludes keep broad scans quiet. Pass an explicit file or
  subdirectory when an excluded tree is the target. Use `--with-excluded` to
  include files matched by contextmink exclude globs, and `--with-git-ignored`
  only for files hidden by Git or `.ignore` rules. Broad scans cross nested Git
  repository roots by default, including tracked submodules and Git-ignored
  sibling repositories, and disclose the exact `nested_repos_entered_total`
  plus a bounded `nested_repos_entered_sample`. Pass `--skip-nested-repos` to
  stay inside each explicit root; pass a nested repository explicitly when it
  is the target.
- Read the `contextmink.receipt.v2` envelope structurally. `scope_complete:
  false` means totals cover only a bounded subset; `output_truncated: true`
  means the scope was inspected but payload was omitted or shortened. Inspect
  `caps[]` for the `boundary`, `dimension`, and `limit`, and use `result.unit`,
  `result.shown`, `result.total`, and `result.total_is_lower_bound` together.
  Use `--fail-if-truncated` when complete displayed output is required or
  `--require-complete-scope` when bounded evidence is unacceptable. Candidate
  enumeration totals stay exact, while grep match totals become lower bounds
  under content-file, content-byte, matching-file, or oversized-file scope
  caps. A no-match grep with
  `no_match_scope: "scanned_subset"` or a `json-select` with `all_null_fields`
  entries needs a narrower or corrected query, not a conclusion.
  JSON fields use literal keys or JSON Pointers (`/result/total`, not dotted
  shorthand). Pass `json-find` match paths directly to `json-select --at`;
  combine `--at /result --keys` to discover a nested object. JSONL pointers
  begin with the zero-based non-empty record index, such as `/12/result`.
- Direct commands are fine when output is already known to be small or
  structurally bounded: `git status --short`, `git diff --stat`, a focused
  test command, a domain tool that emits compact records, or one exact file
  region already known to fit a slice window (about 120 lines). Above that,
  the read is reconnaissance — go through `outline`/`grep`/`slice`. Knowing
  the range you chose does not make the output small; choosing a large range
  is the failure the caps exist to catch.
- Keep domain-specific parsing, validation, indexing, diagnostics, and
  synchronization in project-native tools.
