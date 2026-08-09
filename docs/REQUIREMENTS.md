# Product Requirements

## Primary flow

```text
Paste URL → Analyze → Choose output → Add download → Progress → Completed file
```

## URL analysis
- Accept arbitrary URL text.
- Delegate support detection to yt-dlp.
- Show source/extractor as returned metadata.
- Distinguish single media and playlist where possible.
- Normalize raw yt-dlp formats into human-friendly options.

## Download
- Video mode.
- Audio mode.
- Best or selected quality.
- Container preference.
- Codec preference when meaningful.
- Metadata / thumbnail / subtitles options.
- Output directory.
- Persistent queue.
- Configurable concurrency.
- Stop.
- Resume stopped/failed partial download via yt-dlp continuation.
- Retry failed jobs.

## Progress
Show real:
- state;
- percent when known;
- downloaded bytes;
- total bytes when known;
- speed;
- ETA;
- post-processing state.

## Completed
- List completed files.
- Show metadata and size.
- Download file to LAN client.
- Browser preview where natively supported.
- HTTP Range support.
- No transcoding fallback: show Download/Open when unsupported.

## History
Persist completed, failed and stopped jobs.

## Runtime Manager
Manage:
- yt-dlp
- FFmpeg
- FFprobe

Support:
- missing-component install;
- version inspection;
- update;
- repair/reinstall;
- health checks;
- real progress/status in UI.

## Web UI
- Same UI on localhost and LAN.
- Responsive desktop/mobile layouts.
- No clipboard watcher requirement.

## Network
- Configurable bind address.
- Configurable port.
- Configurable allowed CIDRs.
- `192.168.0.0/16` may be the default LAN allow-list.
- Explicit warning that there is no application login.

## Logs
- application logs;
- yt-dlp stdout/stderr;
- FFmpeg output where relevant;
- human-readable errors in main UI;
- raw details in Logs.

## Non-goals
Not required unless later approved:
- cloud backend;
- accounts/auth/pairing;
- media collections/watch history;
- HLS;
- transcoding server;
- GPU transcoding;
- subscriptions/channel monitoring;
- clipboard monitoring;
- custom website extractors.
