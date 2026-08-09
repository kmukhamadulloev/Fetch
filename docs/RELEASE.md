# Release

## Initial targets
- Windows x86_64
- Linux x86_64
- Linux aarch64
- macOS arm64
- macOS x86_64 where practical

## Build sequence

```text
frontend install/check/test/build
→ embed frontend assets
→ cargo build --release
→ package
→ checksums
→ publish
```

Release runtime requires no Node.js.

## Version independence

```text
Fetch 1.x
Managed yt-dlp independent version
Managed FFmpeg independent version
```

A yt-dlp update must not require a Fetch release.

## Artifacts
Initially archive-based artifacts are acceptable. Installers may be added later.

## Signing
Before broad public distribution, review Windows code signing and macOS Developer ID/notarization.

## Licenses
Before distributing FFmpeg builds, document the exact provider/build/license and include required notices/Open Source Licenses.
