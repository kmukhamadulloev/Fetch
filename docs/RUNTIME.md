# Runtime Manager

Fetch manages yt-dlp, FFmpeg, and FFprobe independently from the application
version. A system executable can satisfy discovery, but managed executables
under the application-data directory take precedence.

```text
fetch-data/runtime/
├── yt-dlp/yt-dlp[.exe]
└── ffmpeg/
    ├── ffmpeg[.exe]
    ├── ffprobe[.exe]
    ├── FFMPEG-LICENSE.txt
    └── PROVIDER.txt
```

## Sources and safety

- yt-dlp uses the official latest standalone asset and official
  `SHA2-256SUMS` document.
- FFmpeg and FFprobe use matching platform assets from the latest
  `eugeneware/ffmpeg-static` GitHub release. GitHub-published SHA-256 asset
  digests are mandatory.
- Supported runtime targets are Linux x86_64/aarch64, macOS x86_64/arm64, and
  Windows x86_64.

Artifacts are downloaded to temporary files, checksum-verified, marked
executable where applicable, and version-checked before replacement. Existing
files are renamed to backups. A replacement or post-install health failure
restores the last working file or FFmpeg/FFprobe pair. Runtime operations are
serialized to prevent first-run and UI actions from racing.

The runtime manager broadcasts actual installing/updating/ready/failed state
over SSE. yt-dlp automatic update is checked at most daily when enabled.
Install, update, and repair are also available in Settings. Fetch does not fake
runtime versions or progress.

Provider requests have bounded connection, idle-read, and total operation
durations. A failed update never demotes an already verified executable: Fetch
restores its Ready state and records the update as a warning. A failed initial
install remains Failed with a retry action. Retained runtime lifecycle records
include the component, operation, source, request stage, failure category, and
underlying cause without returning internal details through normal API errors.

## JavaScript challenge runtimes

Some yt-dlp extractors require a supported external JavaScript runtime. Fetch
does not manage these executables, but discovers `deno`, `node`, and `qjs` from
the process `PATH` and reports their versions through Runtime settings and
`GET /api/runtime/javascript`.

The persisted `ytdlp_js_runtime` setting accepts `auto`, `deno`, `node`,
`quickjs`, or `disabled`. Automatic is the backward-compatible default and
passes Deno, Node, and QuickJS to yt-dlp in its supported priority order.
Explicit modes clear yt-dlp defaults and enable exactly one runtime; disabled
clears all JavaScript runtimes. The adapter passes each value as a structured
process argument and never invokes a shell.

A saved change hot-applies to media analysis and to queued or newly spawned
downloads. An active yt-dlp process keeps the policy it started with. A runtime
installed after Fetch starts is visible after pressing Refresh because
discovery is performed on demand. Desktop launch mechanisms may expose a
different `PATH` than an interactive shell, so the UI reports what Fetch itself
can see rather than assuming a machine-wide installation is reachable.
