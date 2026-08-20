# Fetch Roadmap

This file tracks approved forward-looking product work. Keep completed release
details in `RELEASE.md`, concrete defects in `ISSUES.md`, and verification
evidence in `docs/ACCEPTANCE.md`.

## 0.1.2 — Released

Objective: make the completed-media library organized and resumable while
improving the live settings experience.

### Implemented

- [x] Apply download defaults, concurrency, allowed networks, bind address, and
  port changes without restarting Fetch.
- [x] Hand the browser over to a replacement listener after a successful bind
  or port change, with rollback when the replacement cannot start.
- [x] Add consistent keyboard focus treatment and player shortcuts for seek,
  volume, and fullscreen.
- [x] Update the Completed view immediately when a download finishes.
- [x] Represent completed playlists as stacked cards that open focused,
  ordered media galleries.
- [x] Persist shared watch progress, resume playback, offer Start over, and show
  progress lines on media and playlist cards.
- [x] Keep Completed media and playlist cards fully contained and usable on
  narrow mobile screens.
- [x] Add native tray controls, bounded graceful Quit, and host-only per-user
  system startup with a headless fallback.

### Release gates

- [ ] Complete final user acceptance and visual-polish review on desktop and
  mobile.
- [ ] Run the native package and startup smoke tests for every release target.
- [ ] Verify tray creation, actions, system-start registration, and graceful
  Quit on interactive Windows, macOS, and Linux desktops.
- [ ] Confirm version metadata, release notes, documentation, and artifacts are
  aligned before creating the release commit and tag.

## 0.1.3 — Release candidate

Objective: add safe, understandable outbound proxy routing for media analysis
and downloads without changing Fetch's local/LAN server model.

### Proxy support

- [x] Add host-managed **System default**, **Direct connection**, and
  **Custom proxy** modes under Network settings.
- [x] Support validated unauthenticated HTTP, HTTPS, SOCKS4, and SOCKS5 proxy
  URLs without exposing arbitrary yt-dlp arguments.
- [x] Apply the selected route consistently to media/playlist analysis and
  every newly spawned yt-dlp download process; active processes keep their
  existing route.
- [x] Keep proxy configuration host-only. LAN clients may initiate downloads
  through the host's configured route but cannot read or change its endpoint.
- [x] Persist proxy configuration separately from the LAN-readable application
  settings contract and hot-apply it without restarting Fetch.
- [x] Pass proxy values as structured process arguments, redact them from
  retained diagnostics, and return actionable validation/connection errors.
- [x] Cover validation, persistence, command construction, hot application,
  host/remote authorization, UI states, and desktop/mobile behavior with tests.
- [x] Update the API contract, security model, UI specification, testing docs,
  acceptance matrix, and release notes as implementation lands.

The approved design and acceptance details are in `docs/PROXY.md`.

### 0.1.3 stabilization

- [x] Keep an existing healthy runtime available when an automatic update
  fails, without returning the first-run setup flow.
- [x] Bound stalled managed-runtime provider requests and retain actionable
  operation, component, and low-level failure details.
- [x] Add responsive All, Info, Warning, and Error filters to the Logs page.
- [x] Cover runtime fallback, retained diagnostics, and log filtering with
  backend, frontend, and browser regression tests.

### 0.1.3 release gates

- [x] Pass formatting, linting, Rust/frontend tests, production builds, and
  desktop/mobile browser coverage.
- [x] Build, smoke-test, package, and checksum the local Linux x86_64 archive.
- [ ] Pass the tag-triggered native archive matrix for Linux x86_64/aarch64,
  macOS x86_64/arm64, and Windows x86_64.
- [ ] Confirm interactive tray creation, actions, system-start registration,
  and graceful Quit on supported Windows, macOS, and Linux desktops.

## 0.1.4 — Released

Objective: improve extractor compatibility and add an optional, tightly scoped
Telegram remote-control surface without exposing Fetch's local HTTP server or
turning Fetch into a hosted service.

### JavaScript runtime compatibility

- [x] Add a persisted typed yt-dlp JavaScript-runtime policy with Automatic,
  Deno, Node, QuickJS, and Disabled modes.
- [x] Hot-apply the policy to analysis and newly spawned downloads while
  leaving active downloads unchanged.
- [x] Discover supported host runtimes and show actionable availability and
  version information in responsive Runtime settings.
- [x] Cover argument construction, persistence, discovery, API behavior, and
  desktop/mobile controls with automated tests and documentation.
- [x] Align workspace, frontend, lockfile, and OpenAPI version metadata to
  0.1.4; add release notes only after behavior is verified.

### Telegram bot foundation

- [x] Add typed Telegram settings, status, authorization, pending-action, and
  job-notification domain models without coupling `fetch-core` to HTTP.
- [x] Add SQLite migrations for non-secret integration settings, polling
  offset, expiring analysis confirmations, and job notification ownership.
- [x] Add a secret-store abstraction: native OS credential storage for
  host-entered tokens and a `FETCH_TELEGRAM_BOT_TOKEN` override for headless
  operation. Never store tokens in SQLite or return them through the API.
- [x] Add a Rust-only `fetch-telegram` adapter with typed Bot API requests,
  outbound long polling, bounded timeouts, cancellation, rate-limit handling,
  redaction, and exponential backoff with jitter.
- [x] Start, stop, and hot-reconfigure one bot manager from `app/fetch`; bot
  failure must not affect the HTTP server, downloads, tray, or shutdown.
- [x] Let the host opt Telegram into the shared System, Direct, or Custom
  outbound proxy policy for Bot API tests, polling, replies, and notifications;
  default to direct and reconnect live polling when the selected policy changes.

### Telegram command flow

- [x] Restrict 0.1.4 to allowlisted Telegram user IDs in private chats; ignore
  unauthorized updates without revealing application state.
- [x] Support `/start`, `/help`, `/status`, and `/downloads` with concise,
  rate-limited responses.
- [x] Treat a plain URL message as an analysis request, then present expiring,
  single-use **Download video**, **Download audio**, and **Cancel** actions.
- [x] Require an explicit second confirmation before queueing every item in a
  playlist and show the item count before acceptance.
- [x] Reuse DownloadManager defaults and expose Stop actions for jobs owned by
  the requesting Telegram user; never accept arbitrary yt-dlp arguments.
- [x] Send queued, completed, failed, and stopped notifications without noisy
  per-progress-message updates.
- [x] Optionally stream completed Telegram-owned media back to its private chat,
  enforcing a configurable 1–50 MB ceiling with a safe 50 MB default and a
  text-only fallback for oversized, missing, or failed attachments.
- [x] Persist update offsets and command/job correlations so polling retries or
  process restarts cannot create duplicate downloads.

### Telegram settings and observability

- [x] Add a host-only Integrations settings section for enable/disable, a
  write-only token, allowed user IDs, notification preferences, Test
  connection, bot identity, polling state, last success, and actionable error.
- [x] Keep LAN clients from reading or changing integration configuration;
  display only a host-only explanation to remote browsers.
- [x] Retain redacted lifecycle and command-result diagnostics without message
  bodies, submitted URLs, tokens, or full Telegram identifiers.
- [x] Retain connection-test attempts/results, poller failures/retries, and
  proxy-route reconnects with mode-only diagnostics and bounded test timeouts.
- [x] Document data sent to Telegram and require explicit host acknowledgement
  before the integration can be enabled.
- [x] Gate Telegram fields behind the enable control, summarize missing setup
  requirements, and provide a reveal control for only the unsaved token input.
- [x] Add enable-gated completed-media delivery controls with clear bandwidth,
  privacy, hosted-limit, and oversized-file guidance on desktop and mobile.

### Application identity

- [x] Apply the approved Fetch artwork to Windows executable resources, macOS
  application bundles, Linux startup registration, and installable browser
  metadata.

### 0.1.4 verification and release gates

- [x] Publish curated GitHub Release notes from the matching `RELEASE.md`
  section and reject missing or empty version entries.
- [x] Cover Bot API parsing, authorization, replay/idempotency, persistence,
  secret redaction, backoff, rate limits, cancellation, and service reuse with
  deterministic Rust tests against a fake local Telegram server.
- [x] Cover host/remote settings, token write-only behavior, connection states,
  URL confirmation, playlist confirmation, Stop, and terminal notifications in
  desktop/mobile frontend and browser tests.
- [x] Keep live Telegram testing opt-in and secret-backed; normal CI and release
  acceptance must not require a real bot token or external Telegram access.
- [ ] Pass the full Rust/frontend/browser suite, cross-platform native archive
  matrix, and interactive tray/startup gates before publishing 0.1.4.

The approved behavior, architecture, privacy boundary, milestones, and
acceptance criteria are in `docs/TELEGRAM.md`.

## 0.1.5 — In development

Objective: make the complete Fetch web interface usable in English, Russian,
and Tajik while preserving English as the deterministic default and fallback.

### Localization foundation

- [x] Add a Vue 3 Composition API localization layer with typed `en`, `ru`, and
  `tg` locale catalogs and English fallback behavior.
- [x] Keep language browser-local, default to English without automatic locale
  detection, persist explicit choices, and update the document language live.
- [x] Add an accessible language selector under General settings using English,
  Русский, and Тоҷикӣ labels.

### Complete interface coverage

- [ ] Translate navigation, download creation and progress, completed media,
  history, logs, runtime setup, settings, dialogs, actions, empty/error states,
  confirmations, and accessibility labels.
- [ ] Localize application-owned dates, times, counts, progress summaries, and
  status labels without altering external logs, media metadata, paths, or URLs.
- [ ] Preserve responsive desktop/mobile layouts with longer Russian and Tajik
  labels and change languages without a page reload.

### 0.1.5 verification and release gates

- [ ] Cover English defaulting, invalid-value fallback, persistence, live
  switching, document language, and locale-key completeness with unit tests.
- [ ] Cover language selection and representative translated desktop/mobile
  flows with browser tests.
- [ ] Pass the complete Rust/frontend/browser suite and synchronize README,
  UI, testing, acceptance, issue, roadmap, and release documentation.

## Candidate ideas

These are not commitments and must be promoted into a version before
implementation:

- Search, filter, and sort the Completed library.
- Guarded playlist-level bulk actions.
- Authenticated proxy credentials using native operating-system secret storage.
- Named proxy profiles and per-download proxy selection.
- Optional proxy routing for managed runtime downloads.
- Multiple named Telegram bot profiles.
- Code signing and notarization for native release artifacts.

## Deferred or out of scope

- User accounts, profiles, cloud sync, and application authentication.
- Public internet exposure and hosted services.
- Server-side transcoding or a general-purpose media-server feature set.
