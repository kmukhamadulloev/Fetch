# Release history

Update this file for every Fetch version. Keep entries short and focused on
developer-visible behavior, compatibility, and release operations.

## Unreleased — Media tools and runtime recovery

These additions are implemented locally but have no new release version or tag.

- Added separate Convert and Quick edit workflows from a completed-file menu.
  Saved copies go to Converted/ and Edits/ while preserving originals.
- Added six combinable quick edits: trim, rotate, mute, crop, resize and volume.
  The editor includes native source playback, trim selection, draggable crop,
  tool tabs, presets, reset and a separate export review.
- Renamed Downloads to Processes, retaining the old route and adding persisted
  conversion/edit/metadata jobs, truthful progress, cancellation and retry.
  A compact type dropdown precedes the existing status filters.
- Expanded History to all operation types, including existing persisted records,
  with type filtering, realtime updates and explicit loading/error states.
- Added SQLite migration 0008 for processing jobs and independent derived files;
  downloads and metadata saves retain their existing identities and recovery.
- Added bounded Telegram recovery attempts with 10- and 60-second retry delays,
  an actionable exhausted state, and host runtime restart controls.
- Fixed idle runtime status updates through immediate SSE initialization and
  reconnect/visibility snapshots with stale-response protection.

### CI reliability

- Fixed intermittent Linux runtime-test failures caused by `Text file busy`
  when launching freshly written yt-dlp and JavaScript-runtime test scripts.
  Tests now use immutable executable fixtures and verify yt-dlp health before
  checking failed-update recovery. Production runtime behavior is unchanged.

### Metadata editor

- Added a top-left card menu action opening a responsive audio/video metadata
  modal, with basic fields, collapsed Advanced fields, file information, and
  required cover preview/upload/replacement/removal.
- Added managed FFprobe inspection and FFmpeg stream-copy saves for tested MP3,
  M4A, FLAC, MP4, and MKV tag/artwork combinations without renaming media.
- Added background operation status, conflict detection, recoverable file
  replacement, SQLite library updates, and SSE/thumbnail refresh.
- Localized controls in English, Russian, and Tajik, with keyboard focus and
  unsaved-change guards. Added API, recovery, real-runtime, and browser coverage.
- Added SQLite migration 0007 for metadata recovery journals. Existing files,
  job history, playlist order, and playback progress retain their identifiers.

## 0.1.6 — 2026-09-09

- Added Downloads filters for All, Completed, In Progress, and Error, with
  queued and active stages grouped together and stopped jobs retained under All.
- Added Completed filters for All, Audio, Video, and Playlist. Audio/Video
  include matching playlist members; Playlist retains grouped collections.
- Added locale-aware natural name sorting, newest-first date sorting, and
  Reverse inside a styled Sort by dropdown with icons and keyboard dismissal.
- Positioned connected icon filter groups and sorting in one row beside page
  headings, with horizontal scrolling on phones and clear filtered empty states.
- Upgraded the sidebar runtime summary to labeled system-check bulbs for backend
  connectivity, runtime health, enabled Telegram, and configured Custom proxy.
  Dependent health becomes Unknown when realtime disconnects; proxy configuration
  is not presented as a successful reachability check.
- Localized all new controls and indicators in English, Russian, and Tajik,
  preserving host-only integration access and existing API/database compatibility.
- Added filtering/sorting/playlist browser coverage and status-transition,
  stale-health, and LAN-boundary component tests.
- Updated Linux tray pixel iteration to satisfy current Clippy checks without
  changing icon conversion.

## 0.1.5 — 2026-08-21

- Added browser-local English, Russian, and Tajik language selection with
  deterministic English defaults, live switching, and English fallback.
- Translated the complete responsive interface, settings, player, dialogs,
  statuses, frontend errors, and accessibility labels.
- Added locale-aware dates, numbers, byte sizes, plural counts, catalog-parity
  tests, and desktop/mobile language persistence coverage.
- Kept every Settings section, including Integrations, visibly reachable in a
  responsive tab grid instead of hiding later sections in an unmarked scroller.
- Fixed a production-only blank Integrations route caused by reserved Vue I18n
  message syntax and added catalog-compilation plus production-bundle coverage.

## 0.1.4 — 2026-08-20

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
- Added opt-in shared proxy routing for Telegram with direct-by-default
  behavior, live reconnects, bounded tests, and redacted diagnostics.
- Reworked Telegram setup with enable-gated fields, readiness warnings, and a
  safe show/hide control for newly entered tokens.
- Added opt-in streamed delivery of completed Telegram-owned media with a
  configurable 1–50 MB limit, 50 MB default, and text-only failure fallback.
- Branded Windows executables, macOS application bundles, Linux startup
  entries, and installable browser metadata with the approved Fetch logo.
- Published each tagged version's curated `RELEASE.md` section on its GitHub
  Release page, with validation for missing or empty notes.

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
