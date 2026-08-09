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
- settings/history persistence.

Network:
- CIDR parsing;
- allow/deny decisions.

File serving:
- Range parsing;
- correct 206 ranges;
- invalid ranges;
- opaque file-ID access.

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

## Frontend
Required:
- typecheck;
- lint;
- production build;
- important component/state tests;
- API mapping;
- SSE state updates;
- browser E2E for first run, analyze, add download, progress, completed actions and settings.

## Release acceptance
- cargo fmt;
- clippy;
- Rust tests;
- frontend typecheck/lint/build;
- integration tests;
- target packaging smoke tests.
