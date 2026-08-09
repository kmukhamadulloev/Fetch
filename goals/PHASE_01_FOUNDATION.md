# Phase 01 — Foundation

## Objective
Create the production-grade repository foundation and composition without fake yt-dlp downloads.

## Required work
- Create Cargo workspace boundaries from ARCHITECTURE.md.
- Create real Axum server foundation.
- Add typed `GET /api/status`.
- Add configuration loading.
- Add tracing/logging.
- Add SQLite initialization and migration foundation.
- Create Vue 3 + TypeScript + Vite + Tailwind + Pinia frontend.
- Port approved prototype into reusable Vue components.
- Configure production frontend embedding and SPA fallback.
- Serve same UI on localhost/LAN.
- Add frontend API client abstraction.
- Add development scripts and baseline CI.

## Important
Do not fake media analysis/download behavior. Later-phase actions may be disabled or return a clearly typed NOT_IMPLEMENTED error in Phase 01 only. Never hardcode successful fake media/download/runtime behavior.

## Acceptance criteria
- [ ] `cargo build` succeeds.
- [ ] Workspace boundaries match ARCHITECTURE.md.
- [ ] Axum starts.
- [ ] `/api/status` returns typed JSON.
- [ ] Vue production build succeeds.
- [ ] Vue visually matches approved prototype.
- [ ] Desktop/mobile navigation works.
- [ ] Rust serves embedded production UI.
- [ ] SPA routing does not intercept `/api/*`.
- [ ] SQLite initialization/migration runs.
- [ ] tracing/logging initializes.
- [ ] no forbidden framework is introduced.
- [ ] no fake production media/download implementation exists.
- [ ] format/lint/tests pass.
