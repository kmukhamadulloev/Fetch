# Phase 04 — FFmpeg and Output Options

## Objective
Complete video/audio format behavior and managed FFmpeg/FFprobe runtime.

## Required work
- FFmpeg/FFprobe install/health;
- managed FFmpeg path for yt-dlp;
- format selection mapping;
- video/audio presets;
- container/codec preference;
- audio extraction;
- metadata/thumbnails/subtitles;
- output directory validation;
- expected-output detection;
- UI integration.

## Acceptance criteria
- [ ] missing FFmpeg detected and repairable.
- [ ] yt-dlp uses managed FFmpeg path.
- [ ] video+audio merge works.
- [ ] audio-only output works.
- [ ] UI choices map deterministically to yt-dlp args.
- [ ] output path validated.
- [ ] completed file metadata persisted.
- [ ] tests pass.
