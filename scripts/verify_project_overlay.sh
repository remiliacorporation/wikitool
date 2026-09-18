#!/usr/bin/env bash
set -euo pipefail

# Exercise the archive without installation or Wikitool init. No network calls.
archive="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
if command -v sha256sum >/dev/null 2>&1; then
  hash_command=(sha256sum)
else
  hash_command=(shasum -a 256)
fi
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
project="$fixture/project"
mkdir -p "$project"
unzip -q "$archive" -d "$project"
cd "$project"
suffix=""
[[ -f tools/wikitool/bin/wikitool.exe ]] && suffix=".exe"
wikitool="tools/wikitool/bin/wikitool${suffix}"
planner="tools/papertiger/bin/papertiger${suffix}"
for variable in "${!PAPERTIGER_@}"; do unset "$variable"; done
export PAPERTIGER_ACTOR=overlay-smoke PAPERTIGER_SESSION=overlay-smoke

test ! -e .wikitool
test ! -e state
test -f AGENTS.md
test -f CLAUDE.md
test -f README.md
test -f .gitignore
test -f docs/wikitool/guide.md
for skill in wikitool wiki-writing wiki-interview prose-review contextmink papertiger; do
  test -f ".agents/skills/$skill/SKILL.md"
  test -f ".claude/skills/$skill/SKILL.md"
done
if [[ -n "$suffix" ]]; then
  test -f .agents/skills/contextmink-bridge/SKILL.md
  test -f .claude/skills/contextmink-bridge/SKILL.md
  test -f tools/contextmink/bin/contextmink-bridge.exe
else
  test ! -e .agents/skills/contextmink-bridge
  test ! -e .claude/skills/contextmink-bridge
fi
for skill in wikitool wiki-writing wiki-interview prose-review; do
  diff -qr "tools/wikitool/skills/$skill" ".agents/skills/$skill"
  diff -qr "tools/wikitool/skills/$skill" ".claude/skills/$skill"
done
if [[ -z "$suffix" ]]; then
  for binary in "$wikitool" tools/contextmink/bin/contextmink tools/papertiger/bin/papertiger tools/papertiger/bin/papertiger-mise; do
    test -x "$binary"
  done
fi
"$wikitool" --version
"$wikitool" skills inspect --format json > "$fixture/skills.json"
"$wikitool" config show --format json > "$fixture/config.json"
jq -e '.wiki.url.value == "https://wiki.remilia.org" and .wiki.api_url.value == "https://wiki.remilia.org/api.php" and .adapter_path == "tools/wikitool/site_adapters/remilia-wiki/site-adapter.toml"' "$fixture/config.json"
"$wikitool" companions > "$fixture/companions.json"
jq -e '.status == "ok" and (.companions | length == 2)' "$fixture/companions.json"
"tools/contextmink/bin/contextmink${suffix}" --json files . --limit 2 > "$fixture/retrieval.json"
if [[ -n "$suffix" ]]; then
  printf '#!/usr/bin/env bash\nprintf "bridge-ok\\n"\n' > "$fixture/bridge-check.sh"
  "tools/contextmink/bin/contextmink-bridge.exe" --script "$fixture/bridge-check.sh" > "$fixture/bridge.out"
  grep -Fxq 'bridge-ok' "$fixture/bridge.out"
fi
"$planner" --version
if "$planner" status > "$fixture/planner-status.json" 2> "$fixture/planner-status.err"; then
  echo 'fresh overlay unexpectedly selected existing planning history' >&2
  exit 1
fi
test ! -e state/papertiger.sqlite
test ! -e .wikitool/config.toml

# A local operation materializes directories, never a sync baseline or decision.
"$wikitool" catalog build --format json > "$fixture/catalog.json"
if "$wikitool" status --format json > "$fixture/status.json" 2> "$fixture/status.err"; then
  echo 'fresh overlay incorrectly attested a sync baseline' >&2
  exit 1
fi
grep -Fq 'run a successful `wikitool pull --full --all`' "$fixture/status.err"
test -d wiki_content
test ! -e .wikitool/config.toml
test ! -e .wikitool/acceptance/acceptance.sqlite3

# Project upgrades preserve target, content and authority bytes.
"$planner" init > "$fixture/planner-init.log"
"$planner" plan add work 'Fixture work' --intent 'Verify archive upgrades' > "$fixture/planner-plan.log"
"$planner" add 'Retained outcome' --plan work --intent 'Preserve existing planning history' --intent-source user > "$fixture/planner-task.log"
"$planner" show 1 --json > "$fixture/planner-before.json"
mkdir -p state .wikitool/sync wiki_content/Main
printf '[wiki]\nurl = "https://other.example"\napi_url = "https://other.example/api.php"\n' > .wikitool/config.toml
printf 'durable sync fixture\n' > .wikitool/sync/sync.sqlite3
printf 'existing article\n' > wiki_content/Main/Existing.wiki
"${hash_command[@]}" .wikitool/config.toml state/papertiger.sqlite .wikitool/sync/sync.sqlite3 wiki_content/Main/Existing.wiki > "$fixture/preserved.sha256"
unzip -oq "$archive" -d .
"${hash_command[@]}" -c "$fixture/preserved.sha256"
"$planner" show 1 --json > "$fixture/planner-after.json"
jq -e -s '.[0].task == .[1].task' "$fixture/planner-before.json" "$fixture/planner-after.json"
"$wikitool" config show --format json > "$fixture/existing.json"
jq -e '.wiki.url.value == "https://other.example" and .adapter_path == null' "$fixture/existing.json"

# Companions are optional and an absent binary is reported explicitly.
mv "tools/contextmink/bin/contextmink${suffix}" "$fixture/contextmink${suffix}"
"$wikitool" companions > "$fixture/missing.json"
jq -e '.status == "optional_files_missing" and .wikitool_available == true' "$fixture/missing.json"
"$wikitool" --version
echo 'Extracted project overlay: fresh use, upgrade preservation and optional-companion checks passed.'
