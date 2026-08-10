# UI Specification

## Source of truth

`prototype/fetch-ui/` is the approved visual reference.

Production Vue must preserve its information hierarchy, density, navigation, spacing, dark application styling and responsive behavior.

## Screens
1. New Download
2. Downloads
3. Completed
4. History
5. Settings
6. Logs
7. First-run Runtime Setup

## Desktop
Left sidebar:
- New download
- Downloads
- Completed
- History
- Settings
- Logs

Topbar:
- current screen;
- server/realtime status with primary, shared, reconnecting, and offline states;
- a LAN QR action before Settings for opening Fetch on a phone;
- settings shortcut.

When several same-origin tabs are open, a non-blocking notice identifies the
secondary tab and offers **Make primary**. Taking ownership closes the previous
primary connection; closing a primary tab promotes another automatically.
Server failures show an actionable retry control without hiding already loaded
data.

## Mobile
Bottom navigation:
- New
- Downloads
- Files
- Settings

History and Logs are secondary destinations available from Advanced settings.
Settings sections scroll horizontally on narrow screens, controls keep touch-sized
targets, and overlays account for safe-area insets.

## New Download
Before analysis:
- URL input;
- Analyze button;
- concise runtime/help info.

After analysis:
- preview;
- title/source/duration;
- Video/Audio selector;
- quality;
- container;
- codec preferences;
- extras;
- output path;
- Add to downloads.

Playlist analysis should extend this flow rather than introduce an unrelated visual system.

## Downloads
Show active jobs and queue with human-readable stage, media duration, expected
or transferred bytes, percent, speed, ETA, and Stop/Resume/Retry controls where
appropriate. Queued, downloading, FFmpeg finalization, completed, stopped, and
failed states must explain what Fetch is currently doing.

## Completed
Show cached thumbnail/type, title, MIME type, file size, Watch/Play when
browser-compatible, and a guarded permanent-delete action. On the Fetch host,
the primary file action opens the containing folder; on LAN clients it downloads
the original. Playback uses the same host/remote distinction. Unsupported media
can still open the original stream without attempting transcoding.

The QR dialog lists reachable allowed LAN URLs, generates the QR locally, lets
the user switch addresses or copy one, and repeats the no-authentication warning.
When Fetch is localhost-only it directs the user to Network settings instead of
presenting an unusable loopback QR code.

## Settings
Sections:
- General
- Downloads
- Network
- Runtime
- Advanced

Network includes bind address, port, allowed networks, current URLs and no-auth warning.

Runtime includes yt-dlp and FFmpeg/FFprobe version/status/update/repair controls.

Theme supports Dark, Light, and Use system. The preference is local to each
browser so LAN clients can independently follow their device appearance.

## First run
Show real RuntimeManager install state and retry errors. Never fake production setup progress.

## Visual rules
- graphite dark UI and a restrained light counterpart using the same hierarchy;
- restrained radius around 8–12px;
- thin borders;
- compact controls;
- moderate shadows;
- no giant landing-page typography;
- no excessive gradients;
- no generic SaaS analytics styling;
- monospace for URLs/paths/logs;
- Lucide icons;
- intentional responsive behavior, not just shrinking desktop.
