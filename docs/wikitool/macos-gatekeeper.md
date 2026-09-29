# macOS Gatekeeper and GitHub releases

Wikitool's macOS archives are unsigned GitHub release assets, not an Apple-notarized
application. Every macOS archive contains `tools/wikitool/macos-release-trust.json` with status
`unsigned_github_release`. A browser download and Finder extraction may leave the bundled
executables with the `com.apple.quarantine` attribute. Gatekeeper may then block first launch
of Wikitool, Contextmink or Papertiger with a warning dialog or a `Killed: 9` exit. An updated
archive may carry the attribute again.

Gatekeeper stops a program before its first instruction, so Wikitool cannot clear its own flag.
Instead, the root `AGENTS.md` directs the agent to check for the flag before the first tool
command of a session and, when it is present, to explain it and ask before clearing it. The
user's approval of that one visible command is the gate; no terminal typing is needed.

## What the agent runs

A read-only check, from the project root:

```bash
xattr tools/wikitool/bin/wikitool tools/contextmink/bin/contextmink tools/papertiger/bin/papertiger
```

Each line naming `com.apple.quarantine` identifies a flagged tool. After the user approves,
one command removes that attribute from exactly the flagged files, named by absolute path.
For example, when only Wikitool is flagged:

```bash
xattr -d com.apple.quarantine /absolute/project/tools/wikitool/bin/wikitool
```

`tools/papertiger/bin/papertiger-mise`, Papertiger's optional experimental campaign runner, is
cleared the same way only when a task uses it.

This changes nothing about Gatekeeper for other software. Do not use recursive or blanket forms
(`xattr -r`, `xattr -c`, a directory argument), `sudo`, or `spctl` to disable assessment. A
declined request is not retried.

## Without the agent

A user can instead attempt to run the tool once, then choose System Settings, Privacy &
Security, "Open Anyway" for each blocked executable. Download and extraction methods can
affect whether files carry quarantine; inspect the attribute rather than assuming.

## Provenance

Clearing the flag records the user's decision to run these files; it does not verify them.
Users who want evidence that the archive matches the GitHub release can compare
`shasum -a 256 wikitool-<version>-macos-<arch>.zip` with the matching line in that release's
`SHA256SUMS.txt` before extracting. A checksum copied from inside the archive is not independent
evidence, and neither form supplies an Apple identity.
