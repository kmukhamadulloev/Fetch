# Fetch Roadmap

This file tracks approved forward-looking product work. Keep completed release
details in `RELEASE.md`, concrete defects in `ISSUES.md`, and verification
evidence in `docs/ACCEPTANCE.md`.

## 0.1.2 — Implementation complete; release verification pending

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

## 0.1.3 — Current

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

- [ ] Keep an existing healthy runtime available when an automatic update
  fails, without returning the first-run setup flow.
- [ ] Bound stalled managed-runtime provider requests and retain actionable
  operation, component, and low-level failure details.
- [ ] Add responsive All, Info, Warning, and Error filters to the Logs page.
- [ ] Cover runtime fallback, retained diagnostics, and log filtering with
  backend, frontend, and browser regression tests.

## Candidate ideas

These are not commitments and must be promoted into a version before
implementation:

- Search, filter, and sort the Completed library.
- Guarded playlist-level bulk actions.
- Authenticated proxy credentials using native operating-system secret storage.
- Named proxy profiles and per-download proxy selection.
- Optional proxy routing for managed runtime downloads.
- Code signing and notarization for native release artifacts.

## Deferred or out of scope

- User accounts, profiles, cloud sync, and application authentication.
- Public internet exposure and hosted services.
- Server-side transcoding or a general-purpose media-server feature set.
