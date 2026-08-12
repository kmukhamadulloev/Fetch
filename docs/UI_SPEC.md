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

Playlist downloads appear in the same responsive grid as individual media. A
subtle three-layer offset at the playlist card's top-left, collection icon, and
item count distinguish it without breaking the established card dimensions.
**Open playlist** replaces the grid with a focused gallery whose entries use
the regular media card and original playlist order; **Back to completed**
returns to the mixed library. The selected playlist is represented in the URL,
works across reloads, and updates immediately from completed-file events.
The player provides keyboard controls: Left/Right seek backward or forward five
seconds, Up/Down adjust volume, F enters or exits fullscreen, and Escape closes
the dialog. Shortcuts must not intercept modified commands or editable fields.
Fetch stores shared playback progress in SQLite for browser-playable media.
Positions of at least ten seconds resume automatically; reaching the final five
percent marks an item watched, and watched media restarts from the beginning.
Cards show an accent progress line, playlist cards summarize watched items, and
the player provides **Start over**. Saving is throttled and never interrupts
playback; failures appear as a non-blocking player message.
Completed cards, their action rows, and stacked-playlist decoration remain
inside the content area at 320 CSS pixels and wider. Actions collapse to one
column when two readable touch targets cannot fit.

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
On the Fetch host it also provides System default, Direct connection, and
Custom proxy modes for outbound yt-dlp traffic. Custom mode accepts a validated
unauthenticated HTTP/HTTPS/SOCKS4/SOCKS5 URL. LAN clients see only a host-only
explanation and never request or display the endpoint. Proxy saves have
independent progress, success, validation-error, and retry states.
Download directory, concurrency, and allowed networks apply without restarting.
Listener changes bind the replacement address before retiring the old listener;
the current browser tab then replaces its location with the new URL.

Runtime includes yt-dlp and FFmpeg/FFprobe version/status/update/repair controls.

Theme supports Dark, Light, and Use system. The preference is local to each
browser so LAN clients can independently follow their device appearance.
General settings include **Start Fetch with system**. It applies immediately on
the host, starts Fetch in its tray without opening the browser, and is disabled
with explanatory copy for LAN clients.

## Desktop tray
When supported by the graphical session, the native tray uses the Fetch logo
and offers **Open Fetch**, **Open downloads folder**, and **Quit Fetch**. Opening
Fetch follows the current live listener after port changes. Quit uses bounded
graceful shutdown and stops owned download processes. Tray failure falls back
to browser/terminal operation rather than stopping the server.

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

The Logs view presents retained timestamps, severity, subsystem, summary, and
multiline details in a responsive layout. All, Info, Warnings, and Errors
filters show entry counts and apply immediately without another API request.
Severity is shown as restrained colored text. Optional diagnostic details stay
collapsed behind a right-aligned, keyboard-accessible per-entry accordion.
