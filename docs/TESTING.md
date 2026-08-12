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
- health transitions.

Storage:
- migrations;
- repositories;
- settings/history persistence;
- playlist-context joins and ordering;
- playback-progress migration, persistence, and deletion cascade.

Network:
- CIDR parsing;
- allow/deny decisions.
- cancellation closes active SSE bodies during graceful shutdown.
- proxy mode/URL validation and default compatibility;
- proxy persistence outside application settings;
- matching yt-dlp analysis/download arguments for system, direct, and custom;
- host-only proxy API access and diagnostic redaction.

Desktop lifecycle:
- tray URL follows wildcard, IPv4, IPv6, and live listener changes;
- startup registration follows persisted settings and rolls back on save failure;
- remote clients cannot modify host startup registration;
- graceful shutdown cancels queued/active downloads and owned child processes;
- packaged startup smoke uses `--no-tray` for deterministic headless execution.

File serving:
- Range parsing;
- correct 206 ranges;
- invalid ranges;
- opaque file-ID access.
- opaque cached-thumbnail access and immutable response headers.
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
- browser E2E for first run, analyze, add download, progress, responsive native
  player actions, duration/status detail, host/remote completed actions, guarded
  deletion, stacked playlist galleries, watch progress, resume/start-over,
  LAN QR output, live settings and listener handoff, persisted theme selection,
  host-only system startup, host-only proxy configuration, mobile navigation,
  and multi-tab automatic failover.

Playwright runs the primary flow in desktop and mobile Chromium projects. CI
installs Chromium; locally set `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` when using
an existing Chrome/Chromium installation.

## Release acceptance
- cargo fmt;
- clippy;
- Rust tests;
- frontend typecheck/lint/build;
- integration tests;
- target packaging smoke tests.
