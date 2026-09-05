# Contextmink integration

This repository uses the verified Contextmink source snapshot
`cebecfa65b5e34312f7d2d81b9b46750c0890992` from
https://github.com/remiliacorporation/contextmink.
The binary reports `0.10.0`; this snapshot includes unreleased changes beyond
that published version. Select the source commit, not the version label alone.

`project-install.json` owns the managed integration and selected skill paths.
Ignored host binaries and their hashes live under `bin/`. Repository guidance
and `.contextmink.toml` remain repository-owned.

From this repository root, use `scripts/contextmink` in Bash or
`tools/contextmink/bin/contextmink.exe` in Windows PowerShell. Load the selected
Contextmink skill before broad or potentially high-output reads; detailed
behavior lives in `agent_integration.md` and command help. Project-native
compact queries retain authority. Broad scans follow `.contextmink.toml` and
disclose nested repositories; use `--skip-nested-repos` to stay inside an
explicit root and name a nested project directly when it is the target.

To restore the ignored executables in a fresh clone, use a verified bundle
from the exact source commit above and run its `contextmink setup-project
<repository-root> --dry-run`, inspect the plan, then rerun without `--dry-run`.
Alternatively, check out that commit in the standalone source repository,
run `cargo build --release --locked --bins`, and invoke the resulting
`target/release/contextmink[.exe] setup-project <repository-root>`.
Keep an existing skill selection and valid configuration when restoring.

`json-find` emits JSON Pointers that can be passed to `json-select --at`.
`--at` replaces the former `--array` flag; use `/items/0/name` instead of
dotted discovery paths. The installer owns launcher and reference upgrades.

This development-tool snapshot is separate from `config/contextmink.*`,
which pins the published upstream pack embedded in Wikitool releases.
