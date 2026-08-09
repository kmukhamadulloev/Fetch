# Phase 07 — Runtime Updates and Repair

## Objective
Make runtime maintenance independent of Fetch releases.

## Required work
- runtime manifest abstraction;
- yt-dlp update;
- FFmpeg update strategy;
- checksum verification;
- temporary download/extraction;
- atomic replacement;
- failure safety/rollback;
- repair/reinstall;
- yt-dlp auto-update preference;
- Runtime UI;
- real runtime progress SSE.

## Acceptance criteria
- [ ] failed update cannot destroy working runtime.
- [ ] version rechecked after update.
- [ ] corrupt artifact rejected.
- [ ] UI progress reflects real RuntimeManager state.
- [ ] repair works.
- [ ] app/runtime versions are independent.
- [ ] tests pass.
