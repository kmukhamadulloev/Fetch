# Runtime Manager

Fetch keeps external media tools independent from the Fetch application version.

## Managed components
- yt-dlp
- FFmpeg
- FFprobe

## Logical data layout
Use OS-appropriate application data directories, conceptually:

```text
fetch-data/
├── config/
├── data/fetch.sqlite3
└── runtime/
    ├── yt-dlp/<executable>
    └── ffmpeg/{ffmpeg,ffprobe}
```

## First run

```text
detect OS/arch
→ locate runtime
→ inspect components
→ install missing components
→ health check
→ runtime ready
```

## Safe install/update
Never delete the last known-working component before validating the replacement.

```text
download temporary artifact
→ verify checksum/metadata
→ extract temporary files
→ execute version/health check
→ atomic replace
→ clean previous backup when safe
```

A failed network/update operation must not leave Fetch without a working runtime.

## yt-dlp
Support:
- initial install;
- version check;
- update;
- repair/reinstall;
- automatic update preference;
- optional channel selection if implemented.

Using yt-dlp's supported self-updater is acceptable if wrapped with proper error handling, post-update verification and fallback/rollback behavior.

## FFmpeg
Do not download FFmpeg from arbitrary search results. Use explicit trusted provider manifests per platform/architecture. Provider and license must be documented before release.

## Manifest abstraction
Runtime source metadata should be data-driven with conceptual fields:
- component
- platform
- architecture
- version
- URL
- archive format
- expected files
- checksum

Do not scatter platform URLs throughout business logic.

## Health check
A component is healthy when expected files exist, executable startup/version succeeds and companion files are present.

## UI
Expose real:
- component name;
- version;
- status;
- install/update progress;
- update/repair actions;
- errors.

Do not fake progress.
