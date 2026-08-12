# Fetch Roadmap

This file tracks approved forward-looking product work. Keep completed release
details in `RELEASE.md`, concrete defects in `ISSUES.md`, and verification
evidence in `docs/ACCEPTANCE.md`.

## 0.1.2 — Current

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

## Planned

No later release scope is approved yet. Add work here only after its product
scope and acceptance criteria are agreed.

## Candidate ideas

These are not commitments and must be promoted into a version before
implementation:

- Search, filter, and sort the Completed library.
- Guarded playlist-level bulk actions.
- Code signing and notarization for native release artifacts.

## Deferred or out of scope

- User accounts, profiles, cloud sync, and application authentication.
- Public internet exposure and hosted services.
- Server-side transcoding or a general-purpose media-server feature set.
