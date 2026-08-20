# Release

## Supported archives

- Windows x86_64 (`.zip`)
- Linux x86_64 and aarch64 (`.tar.gz`)
- macOS arm64 and x86_64 (`.tar.gz`)

Windows archives contain `fetch.exe` with a multi-resolution embedded icon.
Linux archives contain the `fetch` executable; the tray and managed XDG startup
entry use the Fetch logo, while a raw ELF file may retain the desktop's generic
executable icon. macOS archives contain a standard `Fetch.app` bundle with an
ICNS application icon. Every archive also contains README, MIT license, and
third-party notices and has a `.sha256` sidecar. yt-dlp, FFmpeg, and FFprobe are
not bundled; the checked runtime manager downloads platform assets on first
use.

## Build and publish sequence

```text
npm ci → typecheck/lint/unit/browser tests → Vite production build
→ embedded Rust release build → packaged-binary startup smoke
→ archive + SHA-256 → tagged GitHub release
```

`.github/workflows/release.yml` uses native hosted runners for every target,
including the GitHub `ubuntu-24.04-arm` runner. A tag matching `v*` publishes
only after all five native builds start and answer `/api/status`. Manual runs
produce downloadable workflow artifacts without publishing a release.

Local Linux packaging:

```bash
cd web && npm ci && npm run build && cd ..
cargo build --release --locked --target x86_64-unknown-linux-gnu
./scripts/smoke-release.sh target/x86_64-unknown-linux-gnu/release/fetch
./scripts/package.sh x86_64-unknown-linux-gnu linux-x86_64
```

Release runtime requires no Node.js. Fetch, yt-dlp, and FFmpeg versions remain
independent. Signing/notarization is not currently provided; review Windows
code signing and Apple Developer ID notarization before broad public
distribution.

Native builds include a system tray. Linux uses the session's freedesktop
StatusNotifierItem implementation and requires no GTK application runtime; some
desktop environments require their usual AppIndicator/StatusNotifier extension.
The Linux startup registration installs the branded icon into the current
user's hicolor icon theme. Windows resource compilation and macOS ICNS bundle
creation happen on their native release runners.
Package smoke tests pass `--no-tray` because CI runners are headless. Windows and
macOS release runs must additionally verify tray creation and graceful Quit on
their native interactive desktops before publication.

## Runtime licensing

Fetch downloads official yt-dlp release artifacts and FFmpeg/FFprobe artifacts
from the documented `eugeneware/ffmpeg-static` provider. SHA-256 values are
verified before execution. The FFmpeg provider license and identity are stored
beside each managed installation. See `THIRD_PARTY_NOTICES.md` for sources and
license boundaries.
