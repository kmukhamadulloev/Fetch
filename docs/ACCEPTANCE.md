# Release acceptance

This matrix records the implemented evidence for every phase criterion. Exact
commands and live-smoke observations are recorded in the completion report.

## Processing execution checkpoint — Local verification (2026-09-23)

- PASS — One persisted FIFO worker executes metadata, conversions and edits;
  downloads retain their manager. Queued metadata keeps its existing Saving API.
- PASS — FFmpeg progress is parsed, exports are probed before publication, and
  public process records/SSE omit filesystem paths. Invalid API requests fail.
- PASS — Missing runtimes/stale sources produce terminal errors; queued cancel,
  running cancel, retry identity, queue restart and retained history are tested.
- PASS — Root snapshots, unique no-clobber outputs, source preservation and
  historical linkage are verified with actual exports. Recovery only removes
  uncommitted destinations matching the temporary file's filesystem identity.
- PASS — Actual six-format matrix, MP4/MKV per-stream copy hashes, six individual
  edits, combined edits/dimensions and trim A/V start alignment pass managed tests.
- PASS — Metadata journal/replacement tests and ten desktop/mobile metadata
  browser regressions pass after routing saves through the shared worker.
- NOT APPLICABLE to checkpoint 2 — New Processes/card/menu/modals; those remain
  checkpoints 3–6. Full A1–A10 acceptance is still pending UI and final hardening.

Checks: `cargo fmt --all -- --check`; strict workspace/all-target/all-feature
Clippy; `cargo test --workspace` (122 passed, opt-in runtime tests excluded);
`cargo build --workspace`; `FETCH_METADATA_RUNTIME=/tmp/fetch-metadata-runtime cargo test -p fetch real_managed_ -- --ignored --nocapture`
(five real-runtime tests passed); frontend typecheck/lint/test/build (39 passed);
`FETCH_E2E_PRODUCTION=1 npm run test:e2e -- --grep 'metadata'` in web (10 passed);
`git diff --check`. No native-platform or release verification is claimed.

## Processing domain/schema slice — Local verification (2026-09-23)

This is checkpoint 1 plus the domain/schema portion of checkpoint 2, not delivery
of Convert, Quick edit, or Processes. Remaining work is tracked in GOAL.md.

- PASS — Structured format/quality/edit requests reject unsafe filenames,
  unknown fields, invalid numeric ranges, overflow, and conflicting options.
- PASS — Lifecycle validation disallows completing queued work or reopening
  terminal attempts; metadata cancellation/retry are not advertised.
- PASS — Migration 0008 upgrades populated pre-existing tables with foreign keys
  enabled while retaining file IDs/paths/thumbnails, playback and metadata journals.
- PASS — Independent exported records have nullable download linkage, explicit
  source/origin, no playlist/playback, and survive source deletion.
- PASS — Process records/private recovery options survive database reopen. Atomic
  output registration rolls back when its process update cannot complete.
- PASS — Existing download/library/API/metadata behavior passes regressions;
  managed-runtime metadata saves still preserve originals and artwork correctly.
- NOT APPLICABLE to this slice — New HTTP routes/SSE, scheduler, export adapter,
  cancellation/recovery execution, and conversion/edit UI. These remain required.

Exact final checks:

- `cargo fmt --all -- --check` — PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS.
- `cargo test --workspace` — PASS: 120 passed, 3 opt-in tests ignored.
- `cargo build --workspace` — PASS.
- `FETCH_METADATA_RUNTIME=/tmp/fetch-metadata-runtime cargo test -p fetch real_managed_metadata -- --ignored --nocapture`
  — PASS: both real managed-FFmpeg metadata tests.
- In `web`: `npm run typecheck`, `npm run lint`, `npm test`, `npm run build`
  — PASS: 39 frontend tests and production build.
- In `web`: `FETCH_E2E_PRODUCTION=1 npm run test:e2e -- --grep 'metadata|completed|Completed'`
  — PASS: 22 desktop/mobile browser tests.
- Relative documentation links and `git diff --check` — PASS.

Full-goal gates remain unchecked individually:

- [ ] A1 — New menu and metadata regression together: pending menu implementation.
- [ ] A2 — Application conversion matrix/copy: pending adapter integration.
- [ ] A3 — Six edits and synchronization: pending implementation.
- [ ] A4 — Complete filesystem/library output safety: schema portion verified only.
- [ ] A5 — Processes view, actions, history: pending UI/API/scheduler.
- [ ] A6 — Runtime cancellation, recovery and consistency: pending execution layer.
- [ ] A7 — Processing snapshots/SSE/LAN convergence: pending integration.
- [ ] A8 — New forms/accessibility/localization: pending UI.
- [ ] A9 — Application export real-runtime tests and aligned final docs: pending.
- [ ] A10 — Full final goal suite/export smoke: pending; native/release unexecuted.

## Runtime summary recovery — Local verification (2026-09-23)

- [x] S1 PASS — Idle SSE sends an immediate opening comment without waiting for
  user activity or a keepalive; host-only Telegram filtering still passes.
- [x] S2 PASS — Initial connection, reconnection, and tab visibility reload
  backend/runtime and permitted Telegram/proxy snapshots. Failed reads retry
  automatically while connected; disconnect/stop clear retry timers.
- [x] S3 PASS — Stores share overlapping startup reads. Late REST snapshots
  preserve newer runtime events and Telegram status received before first load.
- [x] S4 PASS — Secondary tabs request the primary's actual connection state;
  lease ownership alone does not claim connectivity. Old-primary messages and
  replaced-stream callbacks cannot overwrite the current state. Takeover and
  automatic failover retain one SSE owner.
- [x] S5 PASS — LAN clients make no host-only snapshot requests. Background
  refresh does not discard unsaved proxy edits or replace loaded forms with
  initial-loading placeholders. Desktop/mobile browser coverage verifies this.
- [x] S6 PASS — Formatting, strict Clippy, Rust/frontend/browser tests, builds,
  and API/architecture/testing/issue documentation are aligned.
- NOT APPLICABLE — Database migrations, media-runtime changes, version bump,
  release publication, or external service smoke tests.

Exact checks:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo test --workspace` — 115 passed, 3 existing opt-in tests ignored.
- `cargo build --workspace` — passed with the production frontend embedded.
- `npm --prefix web run typecheck` — passed.
- `npm --prefix web run lint` — passed without lint warnings.
- `npm --prefix web test` — 39 passed in 17 files.
- `npm --prefix web run build` — passed, including Vue/TypeScript checking.
- `FETCH_E2E_PRODUCTION=1 npm --prefix web run test:e2e` — 56 passed across
  desktop/mobile Chromium.
- `FETCH_E2E_PRODUCTION=1 npm --prefix web run test:e2e -- --grep 'runtime summary|host can configure Telegram|host can hot-apply'`
  — 6 passed after the final unsaved-form protection. Includes recovery with no
  user actions, existing integration controls, and retained
  proxy edits after a visibility-triggered snapshot refresh.
- OpenAPI YAML parsed successfully; `git diff --check` passed.

No known implementation blocker remains. Native Windows/macOS execution and
reproduction in the user's exact browser/network remain unverified.

## Telegram recovery and restart — Local verification (2026-09-23)

- [x] T1 PASS — Startup and established polling use three consecutive attempts:
  immediate, after 10 seconds, and after another 60 seconds; exhaustion stops
  automatic polling requests and publishes Error with the last cause.
- [x] T2 PASS — Successful polling resets the budget. Permanent errors stop
  immediately, and rate-limit retry-after values can extend the scheduled wait.
- [x] T3 PASS — Saved opted-in proxy changes interrupt retry waits and wake
  exhausted workers with a fresh budget. Cancellation interrupts recovery;
  replacement serializes stopping/joining the previous worker.
- [x] T4 PASS — Host-only `POST /api/telegram/restart` uses saved settings;
  disabled integrations cannot be started through it. Polling offsets remain
  persisted, and LAN clients cannot restart or receive private Telegram status.
- [x] T5 PASS — Integrations exposes Restart Telegram, retry countdown, last
  error, and exhaustion text in English/Russian/Tajik. Component tests cover
  live status, countdown cleanup, disabled/busy controls, and the API request;
  production browser tests cover restart and containment on desktop/mobile.
- [x] T6 PASS — Format, strict Clippy, Rust/frontend tests, frontend lint and
  typecheck, production build, and API/subsystem/issue documentation align.
- NOT APPLICABLE — Database migration, version bump, managed media runtime
  changes, or release publication.

Exact checks:

- `FETCH_RUN_E2E=1 ./scripts/check.sh` — release-note tests, `npm ci`, frontend
  typecheck/lint, 32 unit tests, and production frontend build passed. The first
  run stopped at Clippy's test-fixture type-complexity warning; it was fixed with
  a type alias and all remaining steps were executed explicitly below.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo test --workspace` — 114 passed, 3 existing opt-in tests ignored.
- `cargo build --workspace` — passed.
- `FETCH_E2E_PRODUCTION=1 npm --prefix web run test:e2e` — 52 passed; the two
  Telegram cases found an ambiguous Disabled status selector introduced by the
  additional status panel. The selector now targets the setup status explicitly.
- `FETCH_E2E_PRODUCTION=1 npm --prefix web run test:e2e -- --grep 'host can configure Telegram'`
  — both affected desktop/mobile cases passed after the selector correction;
  all 54 browser cases are covered by the full run plus this focused rerun.
- `git diff --check` — passed.

No known implementation blocker remains. The user's actual proxy and a live
Telegram bot were not exercised; transport tests use a local HTTP/proxy fixture
and lifecycle tests use scripted Bot API responses. Native Windows/macOS
execution remains separate release verification.

## Metadata editor — Local verification (2026-09-23)

- [x] A1 PASS — Top-left rounded pencil opens the modal on individual cards,
  restores keyboard focus, and does not trigger playback.
- [x] A2 PASS — Basic/Advanced audio/video fields follow container capabilities;
  FLAC exposes Comment without the aliased Description; file information is
  read-only and unsupported formats cannot save.
- [x] A3 PASS — Managed MP3/M4A/FLAC/MP4/MKV tag write/clear round trips pass.
  Encoded audio/video stream hashes match. MKV subtitles, languages, and
  chapters survive. Unsafe preservation is rejected before replacement.
- [x] A4 PASS — Embedded artwork preview, replacement, retention, and removal
  pass all five formats; real service tests confirm thumbnail synchronization.
- [x] A5 PASS — Stale/concurrent operations are rejected; failure retains original
  bytes; recovery restores uncommitted replacements and cleans committed ones.
- [x] A6 PASS — Real service tests verify title/size/artwork persistence while IDs,
  history linkage, and playback progress survive. SSE completion and library
  refresh, including artwork URL invalidation, are covered by component tests.
- [x] A7 PASS — Desktop/mobile browser checks cover footer containment, dirty
  guarding, keyboard dismissal/focus return, and English/Russian/Tajik labels.
  Screenshots were inspected at desktop and phone sizes.
- [x] A8 PASS — Formatting, strict Clippy, Rust/frontend tests, lint/typecheck,
  production build, browser coverage, and API/documentation alignment pass.

Exact executed checks:

- `FETCH_RUN_E2E=1 ./scripts/check.sh` — passed: release-note tests, `npm ci`,
  frontend typecheck/lint/unit/build, Rust format/Clippy/tests/build, and
  production-preview Playwright. Rust: 108 passed, 3 opt-in tests ignored;
  frontend: 30 passed in 14 files; browsers: 54 passed across desktop/mobile.
- `cargo test -p fetch real_managed_metadata -- --ignored --nocapture` — 2 passed
  with actual checksum-verified managed FFmpeg/FFprobe and synthetic local media.
  The two metadata tests are included among the normal suite's ignored tests;
  the remaining ignored test is the unrelated live Telegram smoke.
- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo test --workspace` — passed independently after final backend review.
- `cd web && npm run typecheck && npm run lint && npm test && npm run build && FETCH_E2E_PRODUCTION=1 npm run test:e2e`
  — passed after thumbnail refresh integration; 30 unit and 54 browser tests.
- OpenAPI YAML parsed successfully; `git diff --check` passed.

Native Windows/macOS runtime execution, native package matrices, and publication
remain release gates; they were not claimed by this Linux validation. The
container restrictions and single-save concurrency model are documented in
`METADATA.md`. No known local implementation blocker remains.

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
