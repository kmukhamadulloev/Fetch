# Architecture

## System overview

```text
Desktop Browser / Phone / Tablet
              │
          HTTP / SSE
              ▼
┌─────────────────────────────────┐
│ Fetch — Rust                    │
│ ├── Axum HTTP server            │
│ ├── REST + SSE                  │
│ ├── fetch-core                  │
│ ├── Download Manager            │
│ ├── Runtime Manager             │
│ ├── SQLite                      │
│ ├── yt-dlp                      │
│ ├── FFmpeg / FFprobe            │
│ └── embedded Vue production UI │
└─────────────────────────────────┘
```

## Rust workspace

### fetch-core
Owns domain types and application rules:
- MediaInfo / MediaFormat
- DownloadRequest / DownloadJob / DownloadStatus / DownloadProgress
- RuntimeComponent / RuntimeStatus
- ApplicationSettings
- typed domain errors

Must not depend on Axum/HTTP/Vue.

### fetch-ytdlp
Owns:
- yt-dlp command construction;
- media inspection;
- download process execution;
- structured/progress output parsing;
- mapping process failures into typed errors.

### fetch-runtime
Owns:
- OS/architecture detection;
- runtime paths;
- install/update/repair;
- checksum validation;
- atomic replacement;
- executable health checks;
- component versions.

### fetch-storage
Owns:
- SQLite connection;
- migrations;
- download/history repository;
- settings repository;
- completed-file thumbnail references;
- runtime metadata repository if useful.

### fetch-server
Owns:
- Axum router;
- HTTP request/response mapping;
- SSE;
- embedded frontend serving;
- LAN allow-list middleware;
- HTTP Range file responses.

It calls application services; it does not invoke yt-dlp directly.

### app/fetch
Composition root:
- configuration;
- tracing;
- SQLite;
- runtime manager;
- adapters/services;
- Axum startup;
- optional default-browser opening.

DownloadManager derives sanitized playlist directories, asks the yt-dlp adapter
to produce UUID-named JPEG artwork, and associates that private cache artifact
with completed-file persistence.

CompletedLibrary owns completed-media lifecycle operations. It resolves opaque
IDs through SQLite, deletes media/artwork without deleting job history, and
delegates host file-manager reveal to a platform adapter. HTTP handlers never
launch file-manager processes or accept filesystem paths.

## Frontend

Production stack:
- Vue 3
- TypeScript
- Vite
- Tailwind CSS
- Pinia
- Lucide Vue

Suggested structure:

```text
web/src/
├── app/api/
├── app/stores/
├── layouts/
├── views/
└── components/
    ├── ui/
    ├── media/
    ├── downloads/
    └── runtime/
```

API calls use relative `/api/...` URLs. Never hardcode localhost.

## Static asset embedding

Development may use Vite.
Release must embed the built frontend into the Rust executable. Node.js is not required at runtime.

SPA fallback serves `index.html` for client routes without intercepting `/api/*`.

## Realtime

Use SSE at `GET /api/events` for progress/status/runtime events. Commands remain ordinary HTTP requests.

The Vue realtime coordinator elects one primary tab per origin using a short
renewable browser-storage lease. Only that tab opens the SSE stream; it relays
typed events to secondary tabs with `BroadcastChannel`. A secondary tab can
explicitly take ownership, and an expired/released lease triggers automatic
failover. Initial and retry REST reads remain independent from realtime and
have bounded request timeouts, so an unavailable SSE stream cannot block data
loading or leave the interface waiting indefinitely. Browsers without
`BroadcastChannel` retain correct realtime behavior with a per-tab SSE fallback.

The composition root owns a shutdown cancellation token shared with the HTTP
state. Ctrl+C cancels that token so every SSE stream ends before Axum waits for
graceful connection shutdown. A five-second upper bound prevents another
long-lived client response from blocking process exit indefinitely.

## Persistence

SQLite only. Use migrations. Media files stay on disk, not in SQLite.

## Process ownership

Long-running yt-dlp processes are owned by a manager abstraction supporting spawn, stdout/stderr observation, stop request, exit detection and job association. Avoid orphaned children where practical on all supported OSes.

## Typed errors

Examples:
- UnsupportedUrl
- AuthenticationRequired
- GeoRestricted
- MediaUnavailable
- RuntimeMissing
- RuntimeCorrupt
- ProcessFailed
- InvalidSettings
- OutputDirectoryUnavailable
- FileNotFound
- NetworkDenied
