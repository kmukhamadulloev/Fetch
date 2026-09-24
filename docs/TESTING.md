# Testing Strategy

## Unit tests

Core:
- state transitions;
- illegal transitions;
- queue concurrency;
- settings validation.

yt-dlp adapter:
- media JSON normalization;
- playlist detection;
- progress parsing;
- error mapping;
- argument construction.

Runtime:
- platform manifest selection;
- checksum verification;
- temporary install/atomic replacement;
- health transitions;
- failed-update fallback to an existing verified runtime;
- actionable failed-install state and bounded provider failures.
- Deno, Node, and QuickJS discovery/version reporting.

Storage:
- migrations;
- repositories;
- settings/history persistence;
- playlist-context joins and ordering;
- playback-progress migration, persistence, and deletion cascade.

Network:
- CIDR parsing;
- allow/deny decisions.
- cancellation closes active SSE bodies during graceful shutdown;
- idle SSE responses immediately send an opening comment without an application
  event, while Telegram events remain filtered for LAN clients.
- proxy mode/URL validation and default compatibility;
- proxy persistence outside application settings;
- matching yt-dlp analysis/download arguments for system, direct, and custom;
- automatic, explicit, and disabled JavaScript-runtime argument construction;
- persisted JavaScript-runtime defaults and queued-job hot application;
- host-only proxy API access and diagnostic redaction.

Desktop lifecycle:
- tray URL follows wildcard, IPv4, IPv6, and live listener changes;
- startup registration follows persisted settings and rolls back on save failure;
- native icon assets have valid PNG, ICO, and macOS bundle metadata, and Linux
  startup registration installs and removes its hicolor icon;
- remote clients cannot modify host startup registration;
- graceful shutdown cancels queued/active downloads and owned child processes;
- packaged startup smoke uses `--no-tray` for deterministic headless execution.

Release automation:
- `scripts/test-release-notes.sh` verifies tag normalization, exact version
  section extraction, and rejection of missing or empty release notes;
- the release publisher consumes only the matching `RELEASE.md` section through
  GitHub CLI's `--notes-file` option.

File serving:
- Range parsing;
- correct 206 ranges;
- invalid ranges;
- opaque file-ID access.
- opaque cached-thumbnail access and revalidating response headers.
- completed media/artwork deletion with retained job history;
- host-only opaque file-manager reveal and remote denial.
- opaque file-ID playback progress retrieval, validation, save, reset, and
  missing-file handling.

Output organization:
- cross-platform playlist path sanitization;
- one-based ordered playlist templates;
- UUID thumbnail output and persistence.

## Deterministic integration tests

A fake yt-dlp script/executable is allowed ONLY in `tests/fixtures`.

It may emulate:
- `--version`;
- inspect success;
- unsupported URL;
- controlled progress;
- failure exit;
- slow process for Stop tests.

It must never ship in production.

Use it to test API -> manager -> adapter, progress events, stop/resume/retry, persistence and failures.

Runtime health tests symlink the checked-in executable `fake-ytdlp.sh` and
`fake-node.sh` from `tests/fixtures` into isolated temporary directories. Do not
rewrite those executables while tests run: concurrent process spawning can make
freshly written scripts intermittently fail with Linux `ETXTBSY` (Text file
busy). The failed-update test first asserts that the existing runtime is Ready,
then verifies that a provider failure preserves its version, path and Ready
state. Run these tests with `cargo test -p fetch-runtime --lib`.

## Real smoke tests

Separate opt-in/CI job using real managed yt-dlp and FFmpeg. This verifies actual command compatibility without making every PR dependent on external websites.

`scripts/smoke-managed-runtime.sh` installs checksum-verified managed tools in a
fresh data directory, analyzes stable public media, creates an audio extraction
job, and verifies the persisted completed file. The scheduled/manual
`runtime-smoke.yml` workflow runs it independently from pull-request CI.

## Frontend
Required:
- typecheck;
- lint;
- production build;
- important component/state tests;
- API mapping;
- bounded API timeout/error mapping;
- unified SSE state updates and cross-tab primary/secondary takeover;
- automatic snapshot refresh on connect/reconnect/visibility, failed-read retry
  cleanup, truthful secondary-tab state, and obsolete stream callback rejection;
- delayed REST snapshots preserve newer runtime and Telegram events;
- browser recovery of the runtime summary without clicks after a failed initial
  read and after reconnection, plus preservation of unsaved proxy edits during
  background refresh;
- browser E2E for first run, analyze, add download, progress, responsive native
  player actions, duration/status detail, host/remote completed actions, guarded
  deletion, stacked playlist galleries, watch progress, resume/start-over,
  LAN QR output, live settings and listener handoff, persisted theme selection,
  host-only system startup, host-only proxy configuration, mobile navigation,
  JavaScript-runtime discovery and hot-apply controls,
  detailed severity log filtering and accordions, and multi-tab automatic
  failover;
- localization catalog parity, deterministic English fallback, browser-local
  persistence, live English/Russian/Tajik switching, document language,
  locale-aware values, and translated-layout containment on desktop and mobile.

Playwright runs the primary flow in desktop and mobile Chromium projects. CI
installs Chromium; locally set `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` when using
an existing Chrome/Chromium installation.

## 0.1.6 browsing and system-check coverage

Playwright covers job status filters, media filters including playlist members,
both sort keys, Reverse inside the custom dropdown, Escape dismissal, filtered
empty-state reset, preserved playlist ordering/navigation, and desktop/mobile
containment. RuntimeSummary component tests cover live backend/Telegram changes,
runtime readiness, loss of trustworthy dependent health after disconnection,
configured-only proxy labeling, endpoint omission, and no host-only requests
from LAN clients. Existing catalog tests validate all new translations.

## 0.1.5 localization coverage

Unit tests recursively compare every Russian and Tajik catalog leaf against the
canonical English catalog. They also verify English defaulting for missing or
invalid saved values, explicit preference persistence, root `lang` updates, and
locale-aware number, byte-size, and timestamp formatting. Every catalog value
is compiled to reject reserved or malformed Vue I18n message syntax.
Playwright changes
from English to Russian and Tajik without reloading, reloads to confirm
persistence, verifies representative translated navigation and settings text,
and checks that longer labels do not create document overflow in either browser
project. External media titles and diagnostic content remain test fixtures and
are intentionally not translated.

When `FETCH_RUN_E2E=1`, the repository check runs Playwright against the built
Vite preview rather than the development server. This verifies the same
optimized frontend assets embedded into release binaries.

## 0.1.4 Telegram coverage

Normal tests use a fake local Bot API transport and synthetic updates. Current
coverage includes typed Bot API parsing, private-chat authorization, command
and callback parsing, update deduplication and restart recovery, atomic
single-use callback consumption, playlist request expansion, job ownership,
secret redaction, rate-limit classification, cancellation, and host-only API
responses. Multipart transport tests verify streamed video fields and preflight
hosted-size rejection; manager tests cover owned-file lookup, configured-size
skips, upload failure fallback, and redacted output. Frontend and Playwright
coverage verifies host/remote settings, blank-after-save token UI, validation,
completed-media controls, visible navigation into Integrations, and mobile
containment. Poller lifecycle, playlist
callback, Stop, and terminal notifications are covered by deterministic tests.

A real Telegram bot is never required for ordinary CI. Any live smoke is
manual or explicitly selected, reads its token from a CI secret/environment,
uses only `getMe` unless a dedicated allowlisted test user is explicitly part
of a manual command-flow run, and does not print or retain the token or message
content.

Run the read-only live bot-identity smoke explicitly with:

```bash
FETCH_TELEGRAM_BOT_TOKEN='...' \
  cargo test -p fetch-telegram tests::live_bot_identity_smoke_is_opt_in -- --ignored --exact
```

## Release acceptance
- cargo fmt;
- clippy;
- Rust tests;
- frontend typecheck/lint/build;
- integration tests;
- target packaging smoke tests.

## Metadata editor

Normal Rust tests cover typed field validation, opaque API IDs, 202 acceptance,
409 conflicts, background failure status, original preservation, concurrent
deletion exclusion, and restart recovery before/after the SQLite commit marker.
Frontend component tests cover basic/advanced fields, dirty guards, patch-only
requests, and operation-correlated SSE completion. Playwright covers desktop
and mobile modal behavior, upload/removal, save failures, unsupported formats,
focus return, and viewport/footer containment.

Run the real managed-runtime acceptance separately:

```bash
cargo test -p fetch real_managed_metadata -- --ignored --nocapture
```

These tests install checksum-verified managed FFmpeg/FFprobe under
`$TMPDIR/fetch-metadata-runtime` and `$TMPDIR/fetch-metadata-runtime-service`
(or `FETCH_METADATA_RUNTIME`), synthesize local
media without contacting media websites, and exercise MP3/M4A/FLAC/MP4/MKV tag
and artwork read/write/clear. They compare encoded audio/video stream hashes,
preserve MKV subtitle language/chapters, and exercise background save through
SQLite/library/artwork updates and failure rollback. No fake executable ships.

## Local processing backend

Run `cargo test -p fetch processing` for deterministic queue, missing-runtime,
stale-source and publication recovery checks. With managed FFmpeg installed,
run `FETCH_METADATA_RUNTIME=/tmp/fetch-metadata-runtime cargo test -p fetch real_managed_processing -- --ignored --nocapture`.
This exercises real conversion/copy/edit output and queue/collision/cancel behavior.
The expanded real suite also verifies rotated anamorphic inputs, retained
artwork/chapters and decoded audio volume. Point FETCH_METADATA_RUNTIME at an
installed managed runtime root containing ffmpeg/ffmpeg and ffmpeg/ffprobe (or
platform equivalents). Metadata smoke tests can prepare the temporary runtime;
processing tests require it to exist. To exercise both suites together:

```bash
FETCH_METADATA_RUNTIME="$HOME/.local/share/fetch/runtime" cargo test -p fetch real_managed_ -- --ignored --nocapture
```

Run `FETCH_RUN_E2E=1 ./scripts/check.sh` for all mandatory local checks including
production desktop/mobile browser tests. Export browser scenarios cover all
three languages, copy-only runtimes, failure retention, duplicate submission,
keyboard/dirty guards, six edit controls, and unsupported preview/format fallback.
Native platform and interactive release checks remain separate.
