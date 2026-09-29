#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

bundle="$fixture/wikitool-test-macos-arm64"
mkdir -p "$bundle/tools/wikitool/bin" \
  "$bundle/tools/contextmink/bin" \
  "$bundle/tools/papertiger/bin" \
  "$bundle/docs/wikitool" \
  "$bundle/tools/wikitool/skills/wikitool"
printf 'fixture\n' > "$bundle/tools/wikitool/bin/wikitool"
printf 'fixture\n' > "$bundle/tools/contextmink/bin/contextmink"
printf 'fixture\n' > "$bundle/tools/papertiger/bin/papertiger"
printf 'fixture\n' > "$bundle/tools/papertiger/bin/papertiger-mise"
cp "$repo_root/docs/wikitool/macos-gatekeeper.md" "$bundle/docs/wikitool/macos-gatekeeper.md"
cp "$repo_root/AGENTS.md" "$bundle/AGENTS.md"
cp "$repo_root/.agents/skills/wikitool/SKILL.md" \
  "$bundle/tools/wikitool/skills/wikitool/SKILL.md"

bash "$repo_root/scripts/declare_unsigned_macos.sh" --bundle-dir "$bundle" >/dev/null

trust="$bundle/tools/wikitool/macos-release-trust.json"
grep -q '"schema": "wikitool.macos-release-trust.v2"' "$trust"
grep -q '"status": "unsigned_github_release"' "$trust"
grep -q '"gatekeeper": "user_approved_exact_quarantine_exception_required"' "$trust"
grep -q '"executables": \["tools/wikitool/bin/wikitool", "tools/contextmink/bin/contextmink", "tools/papertiger/bin/papertiger"\]' "$trust"
grep -q '"optional_executables": \["tools/papertiger/bin/papertiger-mise"\]' "$trust"
grep -q '"first_run": "AGENTS.md#macos-first-run"' "$trust"
grep -q '^## macOS first run$' "$bundle/AGENTS.md"
grep -q '"instructions": "docs/wikitool/macos-gatekeeper.md"' "$trust"

if bash "$repo_root/scripts/declare_unsigned_macos.sh" --bundle-dir "$bundle" >/dev/null 2>&1; then
  echo "unsigned trust declaration unexpectedly replaced an existing declaration" >&2
  exit 1
fi

rm "$bundle/tools/papertiger/bin/papertiger"
rm "$trust"
if bash "$repo_root/scripts/declare_unsigned_macos.sh" --bundle-dir "$bundle" >/dev/null 2>&1; then
  echo "unsigned trust declaration accepted a bundle without Papertiger" >&2
  exit 1
fi

if grep -R -q -- 'xattr\|-dr\|spctl --master-disable' \
  "$repo_root/scripts/declare_unsigned_macos.sh"; then
  echo "unsigned trust declaration script contains a Gatekeeper mutation" >&2
  exit 1
fi

echo "unsigned macOS release declaration test passed"
