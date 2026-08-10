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
