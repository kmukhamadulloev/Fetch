# HTTP API

Machine-readable contract: `docs/openapi.yaml`.

Base path: `/api`.

## Status
`GET /api/status`

## Media analysis
`POST /api/media/analyze`

Request:
```json
{"url":"https://example.com/..."}
```

Returns normalized media/playlist metadata and format choices.

## Downloads
- `GET /api/downloads`
- `POST /api/downloads`
- `GET /api/downloads/{id}`
- `POST /api/downloads/{id}/stop`
- `POST /api/downloads/{id}/resume`
- `POST /api/downloads/{id}/retry`
- `DELETE /api/downloads/{id}`

Deleting job metadata must not silently delete media files unless later explicitly designed.

## Completed files
- `GET /api/completed`
- `GET /api/files/{id}/download`
- `GET /api/files/{id}/stream`

`stream` supports HTTP Range. It does not transcode.

## History
`GET /api/history`

## Settings
- `GET /api/settings`
- `PUT /api/settings`

Validation is server-side.

## Runtime
- `GET /api/runtime`
- `POST /api/runtime/{component}/install`
- `POST /api/runtime/{component}/update`
- `POST /api/runtime/{component}/repair`

## Events
`GET /api/events` (`text/event-stream`)

Expected event families:
- `download.created`
- `download.progress`
- `download.postprocessing`
- `download.completed`
- `download.failed`
- `download.stopped`
- `runtime.installing`
- `runtime.progress`
- `runtime.ready`
- `runtime.failed`
- `runtime.updated`

## Errors
Normal error shape:

```json
{
  "error": {
    "code": "RUNTIME_MISSING",
    "message": "yt-dlp is not installed",
    "details": null
  }
}
```

Do not encode application errors as HTTP 200.
