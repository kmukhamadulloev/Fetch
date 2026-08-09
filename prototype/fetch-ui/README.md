# Fetch UI Prototype

Interactive HTML/CSS/JS prototype for a cross-platform yt-dlp downloader.

## Stack

- HTML
- Tailwind CSS via CDN
- Custom CSS
- Vanilla JavaScript
- Lucide icons via CDN

## Included screens

- New Download
- Media analysis result
- Video/audio mode selection
- Downloads queue
- Completed files
- History
- Settings
  - General
  - Downloads
  - Network
  - Runtime
  - Advanced
- Logs
- First-run runtime bootstrap modal
- Responsive desktop and mobile navigation

## How to run

Open `index.html` in a browser, or use a simple local web server:

```bash
python3 -m http.server 8000
```

Then open:

```text
http://127.0.0.1:8000
```

To force the first-run setup modal:

```text
http://127.0.0.1:8000/?setup=1
```

## Notes for the Vue 3 implementation

The prototype is intentionally backend-free. Replace simulated JavaScript actions with API calls to the Rust backend.

Recommended production frontend stack:

- Vue 3
- TypeScript
- Vite
- Tailwind CSS
- Pinia
- Lucide Vue

Suggested backend API boundary:

- `POST /api/analyze`
- `GET /api/downloads`
- `POST /api/downloads`
- `POST /api/downloads/:id/stop`
- `POST /api/downloads/:id/resume`
- `GET /api/files`
- `GET /api/history`
- `GET /api/settings`
- `PUT /api/settings`
- `GET /api/runtime`
- `POST /api/runtime/:component/update`
- `POST /api/runtime/:component/reinstall`
- `GET /api/events` (SSE)

Keep the web frontend independent from localhost vs LAN. It should always talk to the same origin using relative API URLs.
