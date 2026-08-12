# Release acceptance

This matrix records the implemented evidence for every phase criterion. Exact
commands and live-smoke observations are recorded in the completion report.

## Phase 01 — Foundation

- [x] PASS — `cargo build` succeeds.
- [x] PASS — workspace boundaries match `ARCHITECTURE.md`.
- [x] PASS — Axum starts and shuts down cleanly.
- [x] PASS — `/api/status` returns typed JSON.
- [x] PASS — Vue production build succeeds.
- [x] PASS — Vue preserves the approved prototype hierarchy and density in dark and light themes.
- [x] PASS — Dark, Light, and Use system preferences apply before paint and persist per browser.
- [x] PASS — desktop sidebar and responsive mobile navigation are browser-tested.
- [x] PASS — Rust embeds and serves production UI assets.
- [x] PASS — SPA fallback never intercepts `/api/*`.
- [x] PASS — SQLite initializes and applies versioned migrations.
- [x] PASS — structured tracing initializes at startup.
- [x] PASS — no forbidden framework or runtime was introduced.
- [x] PASS — production media/runtime/download paths use real adapters.
- [x] PASS — format, lint, unit, integration, and build checks pass.

## Phase 02 — Runtime bootstrap and media analysis

- [x] PASS — missing yt-dlp is detected.
- [x] PASS — real yt-dlp is located or installed from official releases.
- [x] PASS — install is checksum-verified, health-checked, and atomic.
- [x] PASS — first-run UI consumes real runtime state and milestones.
- [x] PASS — arbitrary URL text is passed to yt-dlp without site branches.
- [x] PASS — supported URLs normalize into typed `MediaInfo`.
- [x] PASS — unsupported URLs map to typed errors.
- [x] PASS — playlist and single media are distinct typed results.
- [x] PASS — playlist IDs and item thumbnail metadata normalize without site-specific logic.
- [x] PASS — adapter, fixture integration, and live analysis tests pass.

## Phase 03 — Download manager

- [x] PASS — real yt-dlp downloads complete.
- [x] PASS — HTTP routes call application services, never processes.
- [x] PASS — progress comes from machine-formatted process output.
- [x] PASS — semaphore concurrency is enforced by an integration test.
- [x] PASS — Stop terminates the owned process group and persists `stopped`.
- [x] PASS — Resume uses yt-dlp `--continue` and existing partial files.
- [x] PASS — failures and typed codes persist.
- [x] PASS — post-processing has a persisted state and SSE event.
- [x] PASS — SSE updates the Pinia download store.
- [x] PASS — one elected browser tab owns SSE and relays updates to other tabs.
- [x] PASS — explicit takeover closes the old primary and primary-tab closure fails over automatically.
- [x] PASS — REST reads time out with actionable retry UI independently of realtime.
- [x] PASS — restart recovery preserves history and marks interrupted jobs stopped.
- [x] PASS — playlist child jobs retain shared context and use safe ordered folders.
- [x] PASS — download-manager unit and deterministic integration tests pass.
- [x] PASS — analyzed media duration persists on new jobs and is shown with complete stage/size/ETA status.

## Phase 04 — FFmpeg and outputs

- [x] PASS — missing FFmpeg/FFprobe are detected and repairable as a pair.
- [x] PASS — yt-dlp receives the managed FFmpeg directory.
- [x] PASS — yt-dlp owns normal video/audio merge and post-processing.
- [x] PASS — real managed FFmpeg audio-only extraction completed.
- [x] PASS — UI quality/codec/container choices map to deterministic arguments.
- [x] PASS — output directories and reported completed paths are validated.
- [x] PASS — completed file metadata persists in SQLite.
- [x] PASS — FFmpeg argument, output, and managed-runtime tests pass.

## Phase 05 — Completed files and playback

- [x] PASS — file routes accept only opaque persisted UUIDs.
- [x] PASS — completed-file listing works without exposing paths.
- [x] PASS — completed playlists retain collection identity and original item order across reloads and realtime updates.
- [x] PASS — playback position persists in SQLite and resumes across browser sessions and LAN devices.
- [x] PASS — completed cards and playlists expose realtime watch-progress indicators and reset controls.
- [x] PASS — standalone, stacked-playlist, and focused-gallery cards remain contained at 320 CSS pixels.
- [x] PASS — attachment download streams the original file.
- [x] PASS — valid Range requests return exact `206` bytes and headers.
- [x] PASS — compatible browser media can seek through Range responses.
- [x] PASS — unsupported media is never transcoded.
- [x] PASS — responsive native player supports keyboard dismissal and Open/Download actions.
- [x] PASS — UI shows Open and Download fallbacks for unsupported formats.
- [x] PASS — cached thumbnails are persisted, served by opaque ID, and displayed in cards/player.
- [x] PASS — host clients can reveal an opaque completed file in the native file manager.
- [x] PASS — LAN clients retain attachment download and cannot launch host applications.
- [x] PASS — guarded deletion removes media and artwork while retaining job history.
- [x] PASS — thumbnail cache paths are never serialized to clients.
- [x] PASS — opaque-ID and Range integration tests pass.

## Phase 06 — LAN and settings

- [x] PASS — localhost remains allowed and is the default bind.
- [x] PASS — configured LAN CIDRs are allowed.
- [x] PASS — out-of-policy remote addresses return typed `403`.
- [x] PASS — invalid CIDRs cannot be saved.
- [x] PASS — bind/port validation returns typed settings errors.
- [x] PASS — download defaults, concurrency, allow-list, bind address, and port apply without restarting the process.
- [x] PASS — host settings synchronize per-user background startup while LAN clients cannot alter it.
- [x] PASS — system-default, forced-direct, and custom proxy modes persist separately and hot-apply to newly spawned yt-dlp processes.
- [x] PASS — HTTP, HTTPS, SOCKS4, and SOCKS5 proxy URLs are validated while credentials and ambiguous mode/URL combinations are rejected.
- [x] PASS — analysis and downloads receive the same structured proxy arguments without copying endpoints into job or event payloads.
- [x] PASS — proxy endpoints are host-only and redacted from process diagnostics before logging or persistence.
- [x] PASS — host and LAN-disabled proxy UI states pass desktop and mobile browser tests.
- [x] PASS — every supported operational setting and managed runtime action is exposed in the UI.
- [x] PASS — reachable LAN URLs can be copied or encoded locally as a scannable QR code.
- [x] PASS — no authentication or pairing system was introduced.
- [x] PASS — no router exposure, UPnP, or tunnel behavior exists.
- [x] PASS — policy and settings API integration tests pass.

## Phase 07 — Runtime updates and repair

- [x] PASS — failed pair replacement restores both working binaries.
- [x] PASS — failed provider updates keep existing verified runtimes Ready while first-time failures remain actionable.
- [x] PASS — runtime provider requests have bounded connect, idle-read, and total durations.
- [x] PASS — versions are rechecked after replacement.
- [x] PASS — corrupt artifacts and missing provider digests are rejected.
- [x] PASS — UI state/progress is sourced from RuntimeManager SSE.
- [x] PASS — repair reinstalls and validates managed components.
- [x] PASS — Fetch and runtime component versions are independent.
- [x] PASS — runtime manifest, checksum, rollback, and live-provider tests pass.

## Phase 08 — Logs and diagnostics

- [x] PASS — process failures persist an actionable summary and raw diagnostics.
- [x] PASS — runtime lifecycle records retain component, action, source, provider stage, failure category, and cause.
- [x] PASS — API errors omit raw process output and internal stack details.
- [x] PASS — Logs UI lists, refreshes, clears, filters by severity, and expands details accessibly on desktop and mobile.
- [x] PASS — diagnostics check SQLite, output directory, and all runtimes.
- [x] PASS — API and privacy regression tests pass.

## Phase 09 — Hardening and release

- [x] PASS — all mandatory local checks pass.
- [x] PASS — no production mock, stub, TODO route, or fake state remains.
- [x] PASS — architecture audit found no forbidden application component.
- [x] PASS — release executable embeds the UI and requires no Node.js.
- [x] PASS — explicit headless startup and tray-failure fallback preserve server operation.
- [ ] PENDING RELEASE GATE — interactive tray creation, actions, and graceful Quit pass on every native release target.
- [x] PASS — clean-data managed runtime preparation is scripted and live-tested.
- [x] PASS — native target matrix smoke-tests every archive before publication.
- [x] PASS — Fetch and managed-runtime license notices are included.
- [x] PASS — README and release/API/runtime/testing documentation are current.
- [x] PASS — `ISSUES.md` contains no release-blocking open defect.
