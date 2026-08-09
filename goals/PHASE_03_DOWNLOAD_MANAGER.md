# Phase 03 — Download Manager

## Objective
Implement real persistent yt-dlp download queue and realtime progress.

## Required work
- DownloadJob persistence;
- queue/concurrency;
- yt-dlp argument builder;
- process execution;
- stdout/stderr capture;
- progress normalization;
- state machine;
- postprocessing state;
- Stop/Resume/Retry;
- SSE;
- Downloads UI integration.

## Acceptance criteria
- [ ] real downloads work.
- [ ] no direct yt-dlp invocation in routes.
- [ ] progress is real process output.
- [ ] concurrency limit enforced.
- [ ] Stop safely ends job.
- [ ] Resume relies on yt-dlp continuation.
- [ ] failures persist typed errors.
- [ ] postprocessing represented.
- [ ] SSE updates UI.
- [ ] restart preserves history.
- [ ] tests pass.
