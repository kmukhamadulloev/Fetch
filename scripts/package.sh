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
if [[ "$target" == *-apple-darwin ]]; then
  contents="$stage/Fetch.app/Contents"
  iconset="$stage/Fetch.iconset"
  mkdir -p "$contents/MacOS" "$contents/Resources" "$iconset"
  install -m 755 "$binary" "$contents/MacOS/fetch"
  sed "s/@VERSION@/$version/g" app/fetch/assets/macos/Info.plist.in > "$contents/Info.plist"
  sips -z 16 16 docs/assets/logo.png --out "$iconset/icon_16x16.png" >/dev/null
  sips -z 32 32 docs/assets/logo.png --out "$iconset/icon_16x16@2x.png" >/dev/null
  sips -z 32 32 docs/assets/logo.png --out "$iconset/icon_32x32.png" >/dev/null
  sips -z 64 64 docs/assets/logo.png --out "$iconset/icon_32x32@2x.png" >/dev/null
  sips -z 128 128 docs/assets/logo.png --out "$iconset/icon_128x128.png" >/dev/null
  sips -z 256 256 docs/assets/logo.png --out "$iconset/icon_128x128@2x.png" >/dev/null
  sips -z 256 256 docs/assets/logo.png --out "$iconset/icon_256x256.png" >/dev/null
  sips -z 512 512 docs/assets/logo.png --out "$iconset/icon_256x256@2x.png" >/dev/null
  sips -z 512 512 docs/assets/logo.png --out "$iconset/icon_512x512.png" >/dev/null
  install -m 644 docs/assets/logo.png "$iconset/icon_512x512@2x.png"
  iconutil -c icns "$iconset" -o "$contents/Resources/Fetch.icns"
  rm -r "$iconset"
  plutil -lint "$contents/Info.plist" >/dev/null
else
  install -m 755 "$binary" "$stage/fetch"
fi
install -m 644 README.md LICENSE THIRD_PARTY_NOTICES.md "$stage/"
mkdir -p target/release-artifacts
tar -C target/package -czf "target/release-artifacts/$artifact.tar.gz" "$artifact"
if command -v sha256sum >/dev/null 2>&1; then
  (cd target/release-artifacts && sha256sum "$artifact.tar.gz" > "$artifact.tar.gz.sha256")
else
  (cd target/release-artifacts && shasum -a 256 "$artifact.tar.gz" > "$artifact.tar.gz.sha256")
fi
printf '%s\n' "target/release-artifacts/$artifact.tar.gz"
