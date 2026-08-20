# Release history

Update this file for every Fetch version. Keep entries short and focused on
developer-visible behavior, compatibility, and release operations.

## 0.1.4 — In development

- Added automatic and explicit yt-dlp JavaScript-runtime controls.
- Added Deno, Node, and QuickJS host discovery with version/status feedback.
- Hot-applied JavaScript-runtime changes to analysis and queued/new downloads.
- Established typed Telegram integration, authorization, pending-action,
  notification, repository, secret-store, and host-service contracts.
- Added an optional allowlisted private-chat Telegram bot with URL analysis,
  video/audio and playlist confirmation, owned-job Stop, and bounded notices.
- Added native credential storage, a headless token override, hot polling
  lifecycle, redacted diagnostics, and host-only responsive setup controls.
- Streamed Telegram connection status to host settings without periodic API
  polling or form/UI reloads.

## 0.1.3 — 2026-08-18

- Added host-managed system-default, forced-direct, and custom proxy modes.
- Routed yt-dlp analysis and newly spawned downloads through the saved policy.
- Added validated HTTP, HTTPS, SOCKS4, and SOCKS5 endpoint support.
- Kept proxy endpoints out of LAN APIs, job history, SSE, and diagnostics.
- Added responsive proxy controls and host/remote browser coverage.
- Kept verified runtimes ready when automatic provider updates fail.
- Bounded runtime provider requests and retained actionable operation details.
- Added responsive severity filters, counts, and collapsible details to Logs.

## 0.1.2

- Grouped completed playlist downloads into stacked cards and focused galleries.
- Preserved playlist titles and item order across reloads and realtime updates.
- Reused responsive media cards and per-file actions inside every playlist.
- Kept existing databases compatible by deriving collection data from persisted jobs.
- Added shared watch progress, automatic resume, watched state, and start-over controls.
- Added realtime progress lines for media and playlist cards across connected devices.
- Prevented Completed media and playlist cards from overflowing narrow mobile screens.
- Added native tray controls and optional per-user background startup.
- Made tray Quit stop owned downloads and HTTP connections through bounded graceful shutdown.

## 0.1.1

- Added player shortcuts for seeking, volume, fullscreen, and accessible feedback.
- Added immediate Completed-library updates through persisted-file SSE events.
- Added live download-directory and concurrency changes without interrupting active jobs.
- Added in-process bind/port handoff, browser relocation, and rollback on failed binds.
- Standardized keyboard focus styling and expanded desktop/mobile regression coverage.
- Updated settings, realtime, API, OpenAPI, acceptance, and issue documentation.

## 0.1.0 — Initial release

- Shipped the Rust, Axum, SQLite, and embedded Vue application foundation.
- Added managed yt-dlp, FFmpeg, and FFprobe installation, updates, and repair.
- Added media analysis, playlists, queued downloads, progress, stop, resume, and retry.
- Added thumbnails, completed-file playback, Range streaming, reveal, download, and deletion.
- Added responsive desktop/mobile UI, themes, settings, LAN QR access, and multi-tab realtime.
- Added native release archives, checksums, startup smoke tests, and GitHub release automation.
