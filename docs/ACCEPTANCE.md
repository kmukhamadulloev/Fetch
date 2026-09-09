# Release acceptance

This matrix records the implemented evidence for every phase criterion. Exact
commands and live-smoke observations are recorded in the completion report.

## 0.1.6 — Local verification (2026-09-09)

- [x] PASS — Downloads filters cover completed, queued/active, failed, and All.
- [x] PASS — Completed filters cover audio/video files and playlist collections.
- [x] PASS — Name/date sorting, Reverse inside the custom dropdown, Escape
  dismissal, grouped icon controls, and mobile containment pass browser coverage.
- [x] PASS — Playlist navigation retains original item order and filter state.
- [x] PASS — Backend/runtime bulbs and conditional Telegram/Custom proxy rows
  reflect existing stores, mark stale dependent health Unknown, and protect
  host-only data; proxy configuration is explicitly not a reachability check.
- [x] PASS — English/Russian/Tajik catalog parity and compilation remain valid.
- [x] PASS — Workspace, frontend, both lockfiles, and OpenAPI identify 0.1.6.
- [x] PASS — Release notes include all browsing, layout, sort-menu, and status
  summary changes, with no API or database migration.

Exact local checks:

- `./scripts/test-release-notes.sh` — passed.
- `cd web && npm ci && npm run typecheck && npm run lint && npm test && npm run build`
  — passed; 28 tests in 13 files.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed
  after replacing the existing tray pixel loop with typed four-byte chunks.
- `cargo test --workspace` — 101 passed, 1 opt-in live Telegram test ignored.
- `cargo build --workspace` — passed.
- `FETCH_E2E_PRODUCTION=1 npm --prefix web run test:e2e` — 44 passed across
  desktop/mobile Chromium against the built Vite preview.
- `git diff --check` — passed.

The initial `FETCH_RUN_E2E=1 ./scripts/check.sh` stopped at the Clippy warning;
after the fix, format/Clippy/tests/build and production browser checks were run
explicitly to complete every remaining script step.

- [ ] PENDING — Native archive matrix and interactive tray/startup/visual review.
- [ ] PENDING — Release tag and publication.
- NOT APPLICABLE — New endpoints, persistence migrations, external service
  health probes, or live download smoke for these frontend changes.

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
- [x] PASS — automatic JavaScript-runtime policy enables detected Deno, Node,
  or QuickJS for both analysis and downloads without site-specific branches.
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
- [x] PASS — Telegram is direct by default, can opt into the selected proxy
  route, hot-reconnects on changes, and retains mode-only diagnostics.
- [x] PASS — Telegram completed-media delivery is disabled by default, streams
  only owned completed files, enforces a persisted 1–50 MB ceiling with a 50 MB
  default, and uses text-only fallbacks without exposing local paths.
- [x] PASS — JavaScript-runtime selection persists and hot-applies to analysis
  and queued/new downloads while active processes remain unchanged.
- [x] PASS — Runtime settings report detected Deno, Node, and QuickJS versions
  and remain contained on desktop and mobile.
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
- [x] PASS — JavaScript-runtime discovery and typed yt-dlp argument tests pass.

## Phase 08 — Logs and diagnostics

- [x] PASS — process failures persist an actionable summary and raw diagnostics.
- [x] PASS — runtime lifecycle records retain component, action, source, provider stage, failure category, and cause.
- [x] PASS — API errors omit raw process output and internal stack details.
- [x] PASS — Logs UI lists, refreshes, clears, filters by severity, and expands details accessibly on desktop and mobile.
- [x] PASS — desktop log summaries share one baseline and fixed metadata columns across Info, Warning, and Error entries.
- [x] PASS — diagnostics check SQLite, output directory, and all runtimes.
- [x] PASS — API and privacy regression tests pass.

## Phase 09 — Hardening and release

- [x] PASS — all mandatory local checks pass.
- [x] PASS — no production mock, stub, TODO route, or fake state remains.
- [x] PASS — architecture audit found no forbidden application component.
- [x] PASS — release executable embeds the UI and requires no Node.js.
- [x] PASS — Windows resources, the macOS application bundle, Linux startup
  registration, and installed web identity use the approved Fetch artwork.
- [x] PASS — explicit headless startup and tray-failure fallback preserve server operation.
- [ ] PENDING RELEASE GATE — interactive tray creation, actions, and graceful Quit pass on every native release target.
- [x] PASS — clean-data managed runtime preparation is scripted and live-tested.
- [x] PASS — native target matrix smoke-tests every archive before publication.
- [x] PASS — release automation validates and supplies the matching non-empty
  `RELEASE.md` section to GitHub CLI through `--notes-file`.
- [x] PASS — Fetch and managed-runtime license notices are included.
- [x] PASS — README and release/API/runtime/testing documentation are current.
- [x] PASS — `ISSUES.md` contains no release-blocking open defect.

## Phase 10 — Localization

- [x] PASS — English is the deterministic first-run language and fallback for
  invalid or incomplete preferences.
- [x] PASS — English, Russian, and Tajik catalogs have identical leaf-key
  coverage.
- [x] PASS — language selection is browser-local, persists, applies without
  reload, and updates the root document language.
- [x] PASS — navigation, workflows, settings, dialogs, statuses, frontend
  errors, and accessibility labels are translated.
- [x] PASS — application-owned dates, numbers, byte sizes, and plural counts
  use locale-aware formatting.
- [x] PASS — external media metadata, paths, URLs, runtime output, and retained
  diagnostics remain unchanged.
- [x] PASS — translated desktop and mobile layouts remain contained and
  functional in browser tests.
