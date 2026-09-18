# Retired files

This directory holds retired material for review. It is not an active source,
installation, runtime or release input.

- `source/` preserves the tracked files retired during the complete-project
  release cutover, from commit `9fec7389d6c14861772f15cbbba6804d8ddb4279`.
  Native companion packages now own their generated skills, contracts and
  binaries. Workstation harness settings remain in `.claude/settings.local.json`.
- `local/` is ignored by Git. It holds superseded release experiments, temporary
  fixtures and the verified cleanup archive, including the retained originals.
  The archive's file manifest records original paths and SHA-256 values.

Contextmink excludes this tree from ordinary retrieval to avoid treating retired
instructions as current guidance. Releases use explicit payload paths and do
not include this directory. Durable runtime state and active test evidence stay
in their existing locations.
