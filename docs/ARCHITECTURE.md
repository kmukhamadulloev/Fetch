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
