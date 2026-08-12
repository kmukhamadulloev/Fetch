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
- Group completed playlist children into stacked collection cards and retain
  their playlist title and item order.
- Open a playlist as a focused gallery of its regular media cards, with a clear
  route back to the complete library.
- Cache and display available media thumbnails without exposing filesystem paths.
- Download file to LAN client.
- Browser preview where natively supported.
- Persist shared playback position for completed video and audio.
- Resume unfinished playback, display watch progress on media and playlist
  cards, and provide a Start over action.
- HTTP Range support.
- No transcoding fallback: show Download/Open when unsupported.

## History
Persist completed, failed and stopped jobs.

## Output organization
- Keep standalone downloads directly under the configured output root.
- Store playlist children under an ordered, sanitized playlist folder.
- Keep UI thumbnail cache in application data rather than user media folders.

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
- Host settings can enable or disable per-user system startup.
- No clipboard watcher requirement.

## Desktop lifecycle
- Provide a native tray entry when the desktop session supports one.
- Tray actions open Fetch, open the configured download directory, and
  gracefully quit the application.
- A tray failure must not prevent the HTTP server from running.
- System-start launches run in the tray without automatically opening a browser.
- Terminal and headless operation remain available without a tray.

## Network
- Configurable bind address.
- Configurable port.
- Configurable allowed CIDRs.
- `192.168.0.0/16` may be the default LAN allow-list.
- Explicit warning that there is no application login.

## Outbound proxy routing

Fetch provides a host-only global route for yt-dlp analysis and
downloads. It supports system-default, forced-direct, and validated
unauthenticated HTTP/HTTPS/SOCKS4/SOCKS5 proxy modes. It does not proxy the
Fetch listener, LAN file traffic, or managed runtime installation. See
`PROXY.md` for the implementation and security contract.

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
- multi-user profiles, private per-user watch history, or recommendations;
- HLS;
- transcoding server;
- GPU transcoding;
- subscriptions/channel monitoring;
- clipboard monitoring;
- custom website extractors.
