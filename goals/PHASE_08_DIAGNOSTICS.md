# Phase 08 — Logs and Diagnostics

## Objective
Provide useful support diagnostics without polluting normal UX.

## Required work
- structured logs;
- yt-dlp/FFmpeg output capture;
- Logs UI;
- runtime health diagnostics;
- database/download-directory health;
- log retention;
- human summaries plus raw details.

## Acceptance criteria
- [ ] process failures are actionable in logs.
- [ ] normal API errors do not dump internal stack traces.
- [ ] Logs UI works.
- [ ] diagnostics check runtime/database/output directory.
- [ ] tests pass.
