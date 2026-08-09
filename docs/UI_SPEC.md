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
- server/LAN status;
- settings shortcut.

## Mobile
Bottom navigation:
- New
- Downloads
- Files
- Settings

History/Logs may be secondary mobile destinations.

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
Show active jobs and queue with percent, bytes, speed, ETA, state and Stop/Resume/Retry controls where appropriate.

## Completed
Show thumbnail/type, title, format, resolution/bitrate, file size, Watch/Play when browser-compatible and Download.

## Settings
Sections:
- General
- Downloads
- Network
- Runtime
- Advanced

Network includes bind address, port, allowed networks, current URLs and no-auth warning.

Runtime includes yt-dlp and FFmpeg/FFprobe version/status/update/repair controls.

## First run
Show real RuntimeManager install state and retry errors. Never fake production setup progress.

## Visual rules
- graphite/dark application UI;
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
