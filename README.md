# Fetch

Fetch is a local-first, cross-platform web interface for `yt-dlp`.

It is intentionally **not** a cloud service, account system, media platform, or transcoding server.

## Product model

```text
Browser (desktop / phone / tablet)
        │
        │ HTTP / SSE
        ▼
Fetch — single Rust process
├── Axum HTTP server
├── embedded Vue 3 UI
├── Download Manager
├── Runtime Manager
├── SQLite
├── yt-dlp
└── FFmpeg / FFprobe
```

## Mandatory stack

Backend/runtime:
- Rust
- Tokio
- Axum
- Serde
- SQLite
- tracing

Frontend:
- Vue 3
- TypeScript
- Vite
- Tailwind CSS
- Pinia
- Lucide Vue

Runtime components:
- yt-dlp
- FFmpeg
- FFprobe

## Read first

Codex must read, in order:

1. `AGENTS.md`
2. `GOAL.md`
3. `docs/ARCHITECTURE.md`
4. `docs/REQUIREMENTS.md`
5. `docs/UI_SPEC.md`
6. `docs/API.md`
7. `docs/RUNTIME.md`
8. `docs/DOWNLOAD_ENGINE.md`
9. `docs/SECURITY.md`
10. `docs/TESTING.md`
11. `ISSUES.md`

## Approved prototype

`prototype/fetch-ui/` is the authoritative visual reference for the Vue implementation.

## Active work

`GOAL.md` points to the current development phase.
