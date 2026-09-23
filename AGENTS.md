# Fetch Agent Protocol

This file is authoritative for implementation agents.

## Product identity

Fetch is a local-first cross-platform downloader UI powered by `yt-dlp`.

It is NOT:
- a cloud service;
- a multi-user platform;
- a Plex/Jellyfin replacement;
- a transcoding server;
- a website-specific scraper.

## Mandatory architecture

Use:
- Rust as the only runtime backend;
- Axum for HTTP;
- Tokio;
- Vue 3 + TypeScript;
- Tailwind CSS;
- Pinia;
- Lucide Vue;
- SQLite;
- SSE for realtime server-to-client updates;
- managed external `yt-dlp`, `ffmpeg`, `ffprobe` binaries;
- embedded production frontend assets served by Rust.

The same Vue build must work on localhost and LAN using same-origin relative `/api/...` URLs.

## Forbidden unless an explicitly approved goal requires it

Do NOT introduce:
- Electron
- Tauri
- Node.js backend
- Laravel/PHP backend
- nginx
- Caddy
- Redis
- PostgreSQL/MySQL
- Docker as an application runtime requirement
- external SaaS
- cloud storage
- user accounts
- OAuth
- pairing codes
- application authentication
- WebRTC
- HLS infrastructure
- live transcoding
- platform-specific scraper logic
- custom website extractors

Node.js is allowed only for frontend development/build tooling.

## yt-dlp rule

`yt-dlp` is the source of truth for supported media services.

Never add production logic such as `if youtube`, `if tiktok`, `if vimeo`, etc. Service/extractor names returned by yt-dlp may be displayed only as metadata.

## FFmpeg rule

FFmpeg is used for normal yt-dlp media processing: merge, remux, audio extraction/conversion, metadata, subtitles, thumbnails, post-processing.

The approved conversion/quick-edit goal in `GOAL.md` additionally permits explicit,
user-requested FFmpeg processing of completed local files into saved outputs in
`Converted/` and `Edits/` under the configured download directory. This is a
bounded file-export capability, with originals preserved; it does not authorize
live processing for playback or previews. The mandatory runtime/backend stack
and local/LAN access model remain unchanged.

Fetch must NOT become a transcoding server. Browser playback serves compatible completed files with HTTP Range support. If a browser cannot play a file, show Download/Open instead of transcoding it.

## UI rule

`prototype/fetch-ui` is the visual source of truth.

Preserve:
- information hierarchy;
- navigation;
- density;
- spacing;
- dark application character;
- desktop sidebar;
- mobile bottom navigation;
- settings structure;
- interaction flow.

Do not redesign it into a generic SaaS dashboard or marketing page.

## Required separation

Do not execute processes directly in HTTP route handlers.

```text
HTTP
  ↓
Application/Core service
  ↓
DownloadManager / RuntimeManager / repository
  ↓
Adapter
  ↓
OS process / filesystem / SQLite
```

Core domain code must not depend on Axum or Vue.

## No fake production implementation

The following do NOT count as production implementation:
- hardcoded API success payloads;
- fake media metadata;
- fake runtime status/progress;
- fake download progress;
- TODO route handlers;
- static arrays pretending to be persistence;
- mocked production services;
- swallowed errors;
- placeholder buttons where the active goal requires real behavior.

Mocks/fakes are allowed only in tests and fixtures.

### Test fixture clarification

A fake yt-dlp executable/script may exist ONLY under test fixtures to deterministically test command invocation, JSON/progress parsing, failures, state transitions and resume logic. It must never ship in release builds. Separate real smoke tests must exercise the actual managed yt-dlp runtime.

## Autonomy

Do not ask for confirmation for routine implementation decisions already constrained by docs. If several valid implementations exist, choose the simplest one consistent with architecture, document material choices, and continue.

Only stop when a decision would materially change product scope, security model, mandatory stack, network model or data compatibility.

## Work loop

Before coding:
1. Read `GOAL.md`.
2. Read architecture and relevant subsystem docs.
3. Inspect existing code.
4. Inspect `ISSUES.md`.
5. Identify acceptance criteria.

After coding:
1. Format.
2. Lint.
3. Run relevant unit tests.
4. Run relevant integration tests.
5. Run frontend checks/build.
6. Update docs if behavior changed.
7. Update `ISSUES.md`.
8. Report acceptance criteria individually.

## Definition of Done

A feature is complete only when applicable items exist:
- implementation;
- frontend integration;
- API contract;
- error handling;
- logging;
- persistence;
- tests;
- documentation;
- passing acceptance criteria.

"Compiles" is not Definition of Done.

## Completion report

Always report:
- Implemented
- Exact tests executed and results
- Acceptance criteria: PASS/FAIL/NOT APPLICABLE per item
- Remaining real issues/blockers

Do not claim completion with known failing mandatory criteria.
