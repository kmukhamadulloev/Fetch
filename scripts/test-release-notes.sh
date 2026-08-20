#!/usr/bin/env bash
set -euo pipefail

test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT
fixture="$test_root/RELEASE.md"

printf '%s\n' \
  '# Release history' \
  '' \
  '## 1.2.3 — Current' \
  '' \
  '- Added the first feature.' \
  '- Fixed the second feature.' \
  '' \
  '## 1.2.2' \
  '' \
  '- Older release.' > "$fixture"

expected=$'- Added the first feature.\n- Fixed the second feature.'
actual="$(./scripts/release-notes.sh v1.2.3 "$fixture")"
[[ "$actual" == "$expected" ]]
[[ "$(./scripts/release-notes.sh refs/tags/v1.2.3 "$fixture")" == "$expected" ]]

if ./scripts/release-notes.sh v9.9.9 "$fixture" >/dev/null 2>&1; then
  printf 'missing release sections must fail\n' >&2
  exit 1
fi

printf '%s\n' '# Release history' '' '## 1.2.3' > "$fixture"
if ./scripts/release-notes.sh 1.2.3 "$fixture" >/dev/null 2>&1; then
  printf 'empty release sections must fail\n' >&2
  exit 1
fi

printf 'release-note extraction tests passed\n'
