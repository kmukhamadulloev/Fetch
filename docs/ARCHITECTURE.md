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
│ ├── optional Telegram poller    │
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

It consumes shared typed outbound proxy and JavaScript-runtime policies for
analysis and download command construction. The adapter remains the only layer
that translates these policies into yt-dlp arguments; download requests and
HTTP handlers never accept arbitrary process arguments.

### fetch-runtime
Owns:
- OS/architecture detection;
- runtime paths;
- install/update/repair;
- checksum validation;
- atomic replacement;
- executable health checks;
- component versions.
- discovery and version reporting for host-provided Deno, Node, and QuickJS.

### fetch-storage
Owns:
- SQLite connection;
- migrations;
- download/history repository;
- settings repository;
- completed-file thumbnail references;
- runtime metadata repository if useful.

### fetch-telegram

Owns the optional Telegram Bot API transport, typed update/message/callback
serialization, streamed multipart file upload, long polling, timeout/rate-limit
mapping, and token redaction. It does not depend on Axum, SQLite, yt-dlp, or Vue
and contains no download policy.

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
- optional default-browser opening;
- platform tray and per-user startup-registration adapters.

The composition root also owns the optional `TelegramBotManager`. It authorizes
updates and calls the same media-analysis and download application services as
the web interface. It never invokes HTTP routes or processes directly. Its
poller is independently cancellable and cannot make server startup or shutdown
depend on Telegram availability. See `TELEGRAM.md`.

The tray is a thin native lifecycle adapter, not a second frontend. Windows and
macOS use their native tray APIs through `tray-icon`; Linux uses the
freedesktop StatusNotifierItem protocol. It reads the active listener and
download directory from shared composition-root state. Quit cancels the same
token as Ctrl+C and waits for owned download processes and HTTP connections.
Tray initialization failures are logged and leave browser/terminal operation
available. `--no-tray` provides an explicit headless path.

`docs/assets/logo.png` is the source artwork for native application identity.
The Windows build embeds a generated multi-resolution ICO in the executable;
macOS packaging produces a standard `Fetch.app` bundle and ICNS resource; Linux
installs a 256 px hicolor icon with the managed XDG startup entry. The embedded
frontend exposes the same artwork through its favicon and web manifest.

Per-user startup registration is owned by a settings application service:
Windows uses the current-user Run entry, macOS a LaunchAgent, and Linux XDG
Autostart. Registered launches pass `--background`, which suppresses automatic
browser opening without changing the saved manual-start preference.
On Windows, background or direct desktop tray launches hide an app-owned
console, while a console shared with the launching terminal remains visible so
logs and Ctrl+C continue to work.

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

The server sends an immediate SSE opening comment so an idle connection becomes
usable without waiting for a runtime/download event or the 15-second keepalive.
On initial connection, reconnection, and return to a visible tab, the frontend
refreshes backend/runtime snapshots and host-only Telegram/proxy snapshots.
Failed snapshot reads retry after five seconds while connected, with one refresh
in flight; individual stores also share overlapping reads during startup.
Disconnect and teardown cancel the retry timer. Runtime and Telegram
stores preserve newer SSE updates when an older REST response arrives later.

The Vue realtime coordinator elects one primary tab per origin using a short
renewable browser-storage lease. Only that tab opens the SSE stream; it relays
typed events to secondary tabs with `BroadcastChannel`. Secondary tabs request
the current connection state from the elected primary instead of treating its
lease as proof of connectivity. The primary also shares its state on heartbeats;
replaced-stream callbacks and messages from an old primary cannot override it.
A secondary tab can
explicitly take ownership, and an expired/released lease triggers automatic
failover. Initial and retry REST reads remain independent from realtime and
have bounded request timeouts, so an unavailable SSE stream cannot block data
loading or leave the interface waiting indefinitely. Browsers without
`BroadcastChannel` retain correct realtime behavior with a per-tab SSE fallback.

The composition root owns a shutdown cancellation token shared with the HTTP
state. Ctrl+C cancels that token so every SSE stream ends before Axum waits for
graceful connection shutdown. A five-second upper bound prevents another
long-lived client response from blocking process exit indefinitely.
The same bound covers cancellation of queued and active downloads so tray Quit
does not orphan yt-dlp or FFmpeg descendants.

## Persistence

SQLite only. Use migrations. Media files stay on disk, not in SQLite.
Playback progress is stored per opaque completed-file ID and shared by all
allowed clients; deleting a completed file cascades to its progress record.

The proxy policy is stored separately from LAN-readable application
settings and is read or changed only through a host-authorized application
service. See `PROXY.md`.

Telegram configuration, polling offset, pending confirmations, and job
correlations are stored separately from LAN-readable application settings. The
bot token is held by a secret-store adapter or a headless environment override,
never SQLite. Telegram settings application services are host-authorized. The
manager resolves a completed file only through its persisted Telegram job
correlation and applies the configured size policy before the adapter streams
the file.

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

## Completed-media metadata

`CompletedLibrary` delegates metadata operations to the application-owned
`MetadataService`. `metadata_adapter` invokes managed FFprobe/FFmpeg with
structured arguments, preserves encoded media streams, and validates candidate
outputs. The service owns background status, edit/deletion serialization, a
SQLite replacement journal, file recovery, and library SSE updates. The HTTP
layer only resolves typed requests through the service contract. See
`METADATA.md` for format boundaries and recovery semantics.
