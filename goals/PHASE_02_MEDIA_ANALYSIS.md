# Phase 02 — Runtime Bootstrap + Media Analysis

## Objective
Install/locate a real yt-dlp runtime and analyze arbitrary URLs with it.

## Required work
- runtime data directories;
- yt-dlp installation source abstraction;
- yt-dlp health/version inspection;
- real first-run Runtime Setup UI;
- `POST /api/media/analyze`;
- typed media normalization;
- playlist/single detection;
- human-readable error mapping;
- frontend analysis rendering;
- deterministic integration fixture tests;
- separate real yt-dlp smoke test.

## Acceptance criteria
- [ ] missing yt-dlp detected.
- [ ] real yt-dlp can be installed/located.
- [ ] installation is safe/atomic.
- [ ] setup UI reflects real state/progress.
- [ ] arbitrary URL is delegated to yt-dlp.
- [ ] supported URL returns typed MediaInfo.
- [ ] unsupported URL returns typed error.
- [ ] no website-specific extraction logic exists.
- [ ] playlist and single media distinguishable.
- [ ] tests pass.
