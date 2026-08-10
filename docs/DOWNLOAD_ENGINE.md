# Download Engine

## State machine

```text
CREATED → ANALYZING → READY → QUEUED → DOWNLOADING → POSTPROCESSING → COMPLETED
```

Interrupt/failure states:

```text
FAILED
STOPPED
```

Recovery:

```text
STOPPED --resume--> QUEUED
FAILED  --retry-->  QUEUED
```

Do not implement fake OS process pause/suspend as the product model. UI uses Stop + Resume.

## Queue
DownloadManager owns queue, active count, configured concurrency, scheduling, process association and state transitions. HTTP handlers do not schedule jobs themselves.

## Progress
Prefer explicit/machine-readable yt-dlp progress output over scraping arbitrary human text.

Normalize to nullable fields:
- progress_percent
- downloaded_bytes
- total_bytes
- speed_bytes_per_second
- eta_seconds

## Post-processing
A job is not completed until yt-dlp/FFmpeg post-processing is done and the expected output is valid.

## Exit handling
- successful process + expected output -> COMPLETED
- user stop -> STOPPED
- unexpected/nonzero exit -> FAILED

Persist transitions and diagnostics.

## Resume
Rely on yt-dlp continuation/partial files where supported. Do not invent a custom chunk downloader.

## Playlist
Represent item-level progress explicitly (child jobs or equivalent). Avoid one opaque playlist job that hides individual item state.

Each child stores playlist ID, title, and one-based index. DownloadManager
sanitizes the title and ID as cross-platform path components and writes items
under `Playlists/<title> [<playlist-id>]/` using an ordered filename. Client
text is never joined into a path without this normalization.

## Filesystem
Completed file HTTP access uses stored opaque IDs. Never accept arbitrary server file paths from web clients.

yt-dlp writes a JPEG thumbnail to the application-data `thumbnails` cache using
the job UUID. SQLite associates the private cache path with the completed file;
clients receive only `thumbnail_available` and an opaque thumbnail endpoint.
The embed-thumbnail option remains independent and controls artwork embedded in
the media container.
