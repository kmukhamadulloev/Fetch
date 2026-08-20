#!/usr/bin/env bash
set -euo pipefail

release_ref="${1:?usage: release-notes.sh <tag-or-version> [release-file]}"
release_file="${2:-RELEASE.md}"
version="${release_ref#refs/tags/}"
version="${version#v}"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  printf 'invalid release tag or version: %s\n' "$release_ref" >&2
  exit 1
fi
if [[ ! -r "$release_file" ]]; then
  printf 'release history is not readable: %s\n' "$release_file" >&2
  exit 1
fi

if ! notes="$(awk -v wanted="$version" '
  /^##[[:space:]]+/ {
    if (capturing) exit
    heading = $0
    sub(/^##[[:space:]]+/, "", heading)
    split(heading, fields, /[[:space:]]+/)
    if (fields[1] == wanted) {
      found = 1
      capturing = 1
    }
    next
  }
  capturing {
    lines[++count] = $0
    if ($0 !~ /^[[:space:]]*$/) last_content = count
  }
  END {
    if (!found) exit 2
    first_content = 1
    while (first_content <= last_content && lines[first_content] ~ /^[[:space:]]*$/) {
      first_content++
    }
    for (line_number = first_content; line_number <= last_content; line_number++) {
      print lines[line_number]
    }
  }
' "$release_file")"; then
  printf '%s has no section for version %s\n' "$release_file" "$version" >&2
  exit 1
fi

if [[ -z "${notes//[[:space:]]/}" ]]; then
  printf '%s section for version %s is empty\n' "$release_file" "$version" >&2
  exit 1
fi

printf '%s\n' "$notes"
