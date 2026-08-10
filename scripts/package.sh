#!/usr/bin/env bash
set -euo pipefail

target="${1:?usage: package.sh <target-triple> [artifact-label]}"
label="${2:-$target}"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)"
if [[ -z "$version" ]]; then version="0.1.0"; fi
binary="target/$target/release/fetch"
if [[ ! -x "$binary" ]]; then
  printf 'release binary not found: %s\n' "$binary" >&2
  exit 1
fi

artifact="fetch-v${version}-${label}"
stage="target/package/$artifact"
mkdir -p "$stage"
install -m 755 "$binary" "$stage/fetch"
install -m 644 README.md LICENSE THIRD_PARTY_NOTICES.md "$stage/"
mkdir -p target/release-artifacts
tar -C target/package -czf "target/release-artifacts/$artifact.tar.gz" "$artifact"
if command -v sha256sum >/dev/null 2>&1; then
  (cd target/release-artifacts && sha256sum "$artifact.tar.gz" > "$artifact.tar.gz.sha256")
else
  (cd target/release-artifacts && shasum -a 256 "$artifact.tar.gz" > "$artifact.tar.gz.sha256")
fi
printf '%s\n' "target/release-artifacts/$artifact.tar.gz"
