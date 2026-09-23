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

The sidebar runtime summary is a compact System checks list with icon, label,
text state, and colored bulb. Backend uses the live SSE connection (including
the relayed connection in secondary tabs); connecting/reconnecting is amber,
connected green, and offline red. Runtime health follows managed-component
updates. When backend updates are unavailable, dependent health is Unknown.
On the host, enabled Telegram shows its lifecycle state and Custom proxy shows
a blue Configured indicator, with a tooltip clarifying that reachability is
not checked. Direct/System proxy modes are omitted because they do not prove
a proxy is in use. Host integration settings are loaded once at summary mount;
Telegram lifecycle SSE and local settings saves update existing stores. LAN
clients never request host-only integration configuration. No proxy URL or bot
credential is displayed.

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

Downloads offers All (default), Completed, In Progress, and Error filters.
In Progress includes created, analyzing, ready, queued, downloading, and
postprocessing jobs; Error includes failed jobs. Stopped jobs remain under All.

## Completed
Show cached thumbnail/type, title, MIME type, file size, Watch/Play when
browser-compatible, and a guarded permanent-delete action. On the Fetch host,
the primary file action opens the containing folder; on LAN clients it downloads
the original. Playback uses the same host/remote distinction. Unsupported media
can still open the original stream without attempting transcoding.

The library offers All (default), Audio, Video, and Playlist filters. Audio
and Video show matching MIME-type files, including playlist members; Playlist
shows grouped collections. Sort by Name uses locale-aware natural ordering
(A–Z); Date defaults to newest first and uses each collection's latest completed
file. Reverse reverses the selected order. Filters and sorting update with
realtime store changes and remain selected while opening and returning from a
playlist; they reset when leaving the view. The focused playlist retains its
original item order. Empty filtered results explain how to return to All.
Icon filters form a connected segmented group. Controls sit to the right of
the page title and description on desktop. Completed filters and a styled
Sort by dropdown share one horizontal row.
The dropdown contains Name/Date radio options and a Reverse checkbox, supports
outside-click and Escape dismissal, and displays the current sort in its
trigger; on narrow screens this row
scrolls horizontally below the heading without wrapping its controls. All
controls are translated in the supported languages.
These are client-side view controls with no API or persistence changes.

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
- Integrations (0.1.4)
- Advanced

All six sections remain visible without relying on a hidden horizontal scroll:
the selector uses two columns on narrow phones, three columns on intermediate
screens, and the established vertical rail on desktop.

Network includes bind address, port, allowed networks, current URLs and no-auth warning.
On the Fetch host it also provides System default, Direct connection, and
Custom proxy modes for outbound yt-dlp traffic and opt-in Telegram traffic.
Custom mode accepts a validated unauthenticated HTTP/HTTPS/SOCKS4/SOCKS5 URL. LAN clients see only
a host-only explanation and never request or display the endpoint. Proxy saves
have independent progress, success, validation-error, and retry states.
Download directory, concurrency, and allowed networks apply without restarting.
Listener changes bind the replacement address before retiring the old listener;
the current browser tab then replaces its location with the new URL.

Runtime includes yt-dlp and FFmpeg/FFprobe version/status/update/repair controls.
It also includes a JavaScript-runtime selector with Automatic, Deno, Node,
QuickJS, and Disabled modes, plus detected/not-found state and versions for the
host executables. Missing explicit selections are called out inline. Saving
applies to analysis and queued/new downloads without interrupting active work.

The 0.1.4 Integrations section contains the optional Telegram bot card
defined in `TELEGRAM.md`. Configuration is host-only; remote LAN browsers see
an explanation rather than requesting integration state. Enable is the first
control and gates every dependent field. A `runtime-summary` readiness panel
lists missing token, allowlist, and privacy requirements. The token is
write-only; an eye control reveals only the currently typed unsaved value.
Connection testing has independent progress/error feedback, and the card
remains contained in the mobile settings layout. While visible, it
receives live connection state through the shared host-only SSE stream without
polling the settings endpoint or replacing unsaved form edits.
The same enabled fieldset contains an off-by-default **Send completed media**
control and an integer 1–50 MB limit. The limit defaults to 50 MB, is editable
only while delivery is selected, and explains that larger files remain local
and receive a text-only completion notice.

Theme supports Dark, Light, and Use system. The preference is local to each
browser so LAN clients can independently follow their device appearance.
Language supports English, Russian, and Tajik from General settings. English is
the deterministic default and fallback; the browser-local choice applies live,
persists independently on every client, and updates the document language.
Application-owned labels, statuses, errors, dates, numbers, plural counts, and
accessibility text follow the selected language. External media metadata,
paths, URLs, runtime output, and retained diagnostics remain verbatim.
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
collapsed behind a right-aligned, keyboard-accessible per-entry accordion. On
desktop, timestamp, fixed-width severity, subsystem, message, and action columns
share one vertical center so different severity label lengths never shift the
following content.

## Metadata editor

Individual completed-media cards have a permanently visible 44 px rounded three-dot menu
button inset at the top-left of the image or placeholder. It is a sibling of the
play button, so editing cannot trigger playback. Playlist cards remain unchanged.
The Edit metadata menu item opens a modal with the basic audio/video fields and required artwork
controls specified in `METADATA.md`. Advanced starts collapsed. Read-only file
information has a separate disclosure. Save/Cancel stay visible in the footer;
the content scrolls within the viewport. Dirty dismissal requires confirmation,
keyboard focus stays inside the modal and returns to the invoking menu trigger, and
loading/saving/failure states are announced. All form labels and actions are
translated in English, Russian, and Tajik. Unsupported formats are read-only.

## Processes (active local-media goal)

The navigation formerly called Downloads is now Processes at `/processes`;
`/downloads` redirects with query/hash preserved. Its merged list retains the
existing download row and controls, and adds persisted metadata/conversion/edit
rows, type and status filters, elapsed time, real/unknown progress and output
links. Metadata has no cancellation/retry button; exports cancel or retry from
scratch. A `process` query focuses a job. Output links focus the completed card,
including navigating into a source playlist for metadata results.

Reconnect and secondary-tab snapshots refresh jobs and the completed library.
Late snapshots retain newer SSE state instead of reverting progress or dropping
an export that finished during the request. The separate export forms submit these jobs from the Completed card menu.


### Conversion

The individual-card menu offers Edit metadata and Convert, including placeholder
covers and playlist children. Arrow keys navigate; Escape/outside click dismiss.
The separate Convert modal offers installed-runtime formats, video/audio output,
quality or compatible stream copy, safe basename, and explicit preservation
acknowledgement. Outputs go to Converted/ and show a Converted origin badge.
The form traps/restores focus, guards dirty dismissal, disables duplicate submits,
and preserves inputs on failure. Acceptance closes the modal and offers View
process without navigating automatically. Mobile content scrolls above the footer.


### Quick edit

Quick edit is a separate wider dialog with native source playback (never a
rendered edit preview), or Open/Download when playback is unavailable. Trim uses
seconds within the source duration; video also offers clockwise rotation and
optional audio removal. Selected operations form one export into Edits/.
The source extension is preferred when supported; otherwise the user explicitly
chooses a supported format. Output quality, preservation acknowledgement, dirty
and submission handling match Convert. Derived cards show Edited.

Crop controls use even pixel bounds on the oriented source, before rotation.
Resize defaults to keeping the cropped/rotated aspect ratio (height rounds to
an even pixel); changing it requires the explicit stretching checkbox. Both
output axes must be even and at most 7680 pixels. Volume is 0–400%, with 100%
unchanged, and is hidden/excluded when muted. Video-only controls are hidden
for audio. Invalid combinations keep export disabled and explain the constraint.
