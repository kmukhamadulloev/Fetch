# Issues

Maintained by implementation agents. Record real defects, architecture
mismatches, and missing mandatory behavior with an ID, status, affected
subsystem, expected behavior, actual behavior, and notes.

## Release verification

Status: COMPLETE

Phases 01 through 09 were audited against their acceptance criteria. No known
release-blocking implementation defect remains. The authoritative evidence
matrix is `docs/ACCEPTANCE.md`.

## Non-blocking distribution follow-up

Status: DOCUMENTED

Windows code signing and macOS Developer ID signing/notarization are not part of
the current archive release. `docs/RELEASE.md` calls out the required review
before broad public distribution. This does not affect local execution or the
native package startup gates.

## UX-020 — yt-dlp traffic cannot use a host-managed proxy

Status: RESOLVED

Affected:
- fetch-core
- fetch-storage
- fetch-ytdlp
- fetch-server
- app/fetch
- web

Expected: The host can select system-default, forced-direct, or a validated
outbound proxy for media analysis and downloads without exposing proxy
configuration to LAN clients or accepting arbitrary yt-dlp arguments.

Resolution: Added a separately persisted typed proxy policy with host-only API
access and responsive Network settings. HTTP, HTTPS, SOCKS4, and SOCKS5 proxy
URLs hot-apply to analysis and newly spawned downloads; active processes retain
their route. The initial scope rejects credentials, passes arguments without a
shell, and redacts endpoints before retaining subprocess diagnostics.

## BUG-011 — Windows release smoke can fail during temporary cleanup

Status: RESOLVED

Affected:
- release automation

Expected: A Windows archive passes its release gate after the packaged Fetch
binary starts and returns a ready server status.

Actual: Fetch begins managed-runtime bootstrap in the background. The smoke
script force-stopped the ready server and immediately removed its temporary
data directory, allowing a transient Windows file-handle race on the active
`yt-dlp.download-*` file to override the successful smoke result.

Resolution: The Windows smoke script no longer exits from inside its guarded
probe. Cleanup waits for Fetch to terminate and retries directory removal while
Windows releases transient handles. Exhausted best-effort cleanup emits a
warning without converting a verified application startup into a release
failure.

## BUG-016 — Completed cards overflow narrow mobile screens

Status: RESOLVED

Affected:
- web

Expected: Completed media and playlist cards remain fully inside the content
area on supported mobile widths, with readable touch actions.

Actual: Grid children retained intrinsic width from their two-column action
labels, while stacked-playlist decoration could extend the visual footprint.
On narrow phones a card could paint outside its container.

Resolution: Completed grids and cards now explicitly permit their columns and
children to shrink, action labels truncate safely, and action rows collapse to
one column below 360 CSS pixels. A browser regression test verifies standalone,
playlist, and focused-gallery cards at 320 CSS pixels without document overflow.

## UX-019 — Browser-only lifecycle has no persistent desktop control

Status: RESOLVED

Affected:
- app/fetch
- fetch-server
- web
- release automation

Expected: Closing the browser does not make Fetch feel orphaned. Desktop users
can reopen the interface, reach downloads, quit safely, and optionally launch
Fetch automatically after sign-in.

Actual: Fetch opened a browser but exposed no persistent desktop control. Users
had to return to the launching terminal to stop the process, and there was no
system-start option.

Resolution: Added native Windows/macOS tray and Linux StatusNotifier adapters
with Open Fetch, Open downloads folder, and graceful Quit. Quit shares the
Ctrl+C cancellation path and stops owned download processes. Host settings now
synchronize per-user Windows Run, macOS LaunchAgent, or Linux XDG Autostart
registration using a fixed background argument. LAN clients cannot change that
host setting, tray failures preserve browser/terminal operation, and release
smokes use the explicit `--no-tray` headless mode.

## BUG-012 — Tagged release publisher lacks Git tag context

Status: RESOLVED

Affected:
- release automation

Expected: After all native archives pass, the tagged workflow verifies the tag
and publishes one GitHub Release containing every archive and checksum.

Actual: The publish job downloaded build artifacts into an otherwise empty
runner. `gh release create --verify-tag` invokes Git to verify the release tag,
so it failed because no repository had been checked out.

Resolution: The publish job now checks out the triggering ref with complete tag
history before downloading artifacts and invoking GitHub CLI. Tag verification
remains enabled.

## BUG-001 — Ctrl+C waits indefinitely for SSE clients

Status: RESOLVED

Affected:
- app/fetch
- fetch-server

Expected: The first Ctrl+C closes realtime streams and terminates the process
cleanly, even while browser clients are connected.

Actual: Axum stopped listening but waited forever for `/api/events`; pressing
Ctrl+Z then suspended the still-running `cargo run` job.

Resolution: The composition root now propagates a cancellation token into SSE
handlers and applies a five-second graceful-shutdown upper bound. A regression
test consumes an active SSE body and verifies cancellation closes it.

## UX-002 — Frontend theme, settings, player, and mobile gaps

Status: RESOLVED

Affected:
- web

Expected: The production UI provides Dark, Light, and Use system themes,
complete controls for supported settings and managed runtimes, responsive native
playback, and intentional mobile access to primary and secondary destinations.

Resolution: Added per-browser theme persistence with pre-paint application,
completed runtime and network settings interactions, a focus-managed responsive
player with safe fallbacks, horizontally scrollable mobile settings navigation,
secondary History/Logs links, safe-area handling, and desktop/mobile browser
coverage.

## UX-003 — Playlist organization and retained artwork

Status: RESOLVED

Affected:
- fetch-core
- fetch-ytdlp
- fetch-storage
- fetch-server
- app/fetch
- web

Expected: Standalone downloads remain in the selected root, playlist children
use a safe ordered playlist folder, and available artwork remains visible after
analysis without exposing server paths or cluttering the media directory.

Resolution: Playlist child jobs now persist shared ID/title/index context and
DownloadManager creates `Playlists/<title> [<id>]/` using cross-platform path
normalization. yt-dlp and managed FFmpeg cache UUID-named JPEG thumbnails under
application data, SQLite retains the association, and an opaque immutable API
serves artwork to Completed cards and the responsive player. Fake-adapter,
storage, route, argument, sanitization, and desktop/mobile browser tests cover
the behavior; a real yt-dlp JPEG conversion smoke check also passed.

## BUG-004 — Browser can retain a stale embedded frontend after upgrade

Status: RESOLVED

Affected:
- fetch-server
- web

Expected: Reloading Fetch after rebuilding or upgrading the native application
always loads the matching frontend entry document while fingerprinted assets
remain efficiently cacheable.

Actual: Embedded frontend responses did not declare a cache policy. A browser
could reuse an older `index.html`, pairing stale client code with the current
API and presenting as an indefinitely loading or empty application.

Resolution: SPA entry and history-fallback responses now use
`no-cache, no-store, must-revalidate`; Vite-fingerprinted assets use a one-year
immutable cache; other embedded files revalidate. Regression tests verify both
policies, and the frontend now ships an explicit favicon rather than producing
a misleading failed request on every load.

## BUG-005 — Multiple tabs can starve API requests with duplicate SSE streams

Status: RESOLVED

Affected:
- web

Expected: Opening additional Fetch tabs does not consume duplicate persistent
connections or prevent REST data from loading. Users can see which tab owns
realtime delivery, transfer ownership, recover automatically when it closes,
and retry actionable server failures.

Actual: Each tab opened separate download and runtime SSE streams. Enough tabs
could exhaust the browser's per-origin HTTP connection allowance, after which
new API requests appeared to hang without useful feedback.

Resolution: A unified coordinator now elects one primary tab with a renewable
lease, owns one SSE stream for both event families, and relays updates through
`BroadcastChannel`. Secondary tabs expose a Make primary action; takeover closes
the former stream and primary closure triggers automatic failover. REST calls
remain independent, abort after ten seconds, and surface offline/retry UI.
Pinia and desktop/mobile Playwright tests cover event routing, secondary startup,
takeover, connection cardinality, and failover.

## UX-006 — Product artwork is inconsistent across repository and application

Status: RESOLVED

Affected:
- web
- README

Expected: The supplied Fetch artwork consistently identifies the repository,
browser tab, desktop sidebar, mobile header, and first-run experience.

Actual: The repository had no product hero, while the frontend used unrelated
Lucide glyphs and a generated SVG favicon as application marks.

Resolution: The repository README now leads with the approved `repo.png` hero
and presents the product, architecture, output layout, setup, configuration,
security model, and documentation more clearly. The approved `logo.png` now
ships in the embedded frontend and is used for the favicon, Apple touch icon,
sidebar/mobile identity, and runtime setup.

## UX-007 — Mobile connection and media lifecycle actions are incomplete

Status: RESOLVED

Affected:
- fetch-core
- fetch-storage
- fetch-server
- app/fetch
- web

Expected: Users can open the LAN UI without manually typing an address, see
complete download metadata and state, use a native folder action only on the
host, retain Download on remote devices, and deliberately remove completed
media.

Actual: LAN URLs were text-only, completed cards always downloaded another copy,
there was no completed-media deletion flow, duration was discarded between
analysis and job creation, and several download stages lacked useful detail.

Resolution: Added local QR generation from filtered network URLs, host-client
detection, a loopback/host-only opaque reveal operation, and a CompletedLibrary
service that deletes media plus artwork while retaining history. New jobs retain
analyzed duration, and download cards now show human-readable stages, duration,
bytes, speed, progress, and ETA. Confirmation, error, desktop/mobile, API,
filesystem, host/remote, and QR tests cover the complete flow.

## BUG-008 — Mobile bottom navigation is hidden with the desktop sidebar

Status: RESOLVED

Affected:
- web

Expected: The four-item bottom navigation remains visible and usable below the
desktop breakpoint.

Actual: The fixed mobile navigation was rendered inside the desktop sidebar.
Hiding that sidebar below 1024px also hid its complete subtree, including the
mobile navigation.

Resolution: AppNavigation now renders an explicit desktop or mobile variant.
The desktop variant remains in the sidebar while the mobile variant is mounted
at the app-shell root. Unit tests verify the separate destination sets, and the
mobile browser test now checks actual visibility and navigation instead of only
checking the bar's own `display` declaration.

## UX-009 — Mobile top bar prioritizes the page title over connection state

Status: RESOLVED

Affected:
- web

Expected: The compact mobile top bar shows the product mark and complete server
connection state without duplicating the current page title.

Actual: The page title consumed limited horizontal space while the connection
label was hidden, leaving only an unexplained colored dot.

Resolution: Page titles and subtitles are now desktop-only in the top bar. The
full connection label remains visible at mobile widths, with browser coverage
for both responsive states.

## UX-010 — Native select arrows crowd the control edge

Status: RESOLVED

Affected:
- web

Expected: Select controls have a consistent chevron with comfortable spacing
from the right edge in light and dark themes.

Actual: Browser-native select indicators varied by platform and appeared too
close to the rounded control border.

Resolution: Selects now use a consistent lightweight chevron whose visible
stroke aligns with the left text inset after accounting for the SVG's internal
whitespace. They reserve text space for the indicator and expose a clear
disabled state. Browser coverage verifies the normalized appearance and
reserved padding.

## UX-013 — Keyboard focus falls back to inconsistent browser outlines

Status: RESOLVED

Affected:
- web

Expected: Keyboard navigation retains a clear, consistent focus indicator that
matches Fetch in light and dark themes without outlining layout containers.

Actual: Links and other controls could use the browser's white default outline
when reached with Tab, while form controls used a different focus treatment.
Programmatically focused layout containers could also inherit a large outline.

Resolution: Fetch now provides one accent-colored `:focus-visible` ring for
keyboard-focusable controls, suppresses outlines for negative-tab-index layout
targets, and retains the existing border/shadow treatment for text inputs and
selects without adding a duplicate ring. Browser coverage verifies the rendered
keyboard outline color and style.

## UX-014 — Player lacks efficient keyboard controls

Status: RESOLVED

Affected:
- web

Expected: Desktop users can seek, adjust volume, and toggle fullscreen without
leaving the keyboard, with discoverable controls and clear feedback.

Actual: The player exposed only native browser controls and Escape-to-close;
behavior and shortcut discovery varied between browsers.

Resolution: The player now maps Left/Right to five-second seeking, Up/Down to
ten-percent volume changes, and F to fullscreen entry/exit for both video and
audio. Actions are clamped, provide transient accessible feedback, are listed
in the footer, and do not intercept modified shortcuts or editable fields.

## BUG-015 — Open Completed page remains stale after a download finishes

Status: RESOLVED

Affected:
- web

Expected: A completed download appears in the Completed library immediately,
including when that page is already open in a primary or secondary browser tab.

Actual: The realtime `download.completed` event updated only the Downloads
store. The Library store loaded on page mount, so it did not observe files
inserted into SQLite after the Completed view was already mounted.

Resolution: The backend now emits a dedicated `library.completed` event carrying
the completed-file record only after it is persisted. Primary and secondary
tabs merge that record directly into the Library store, while download events
update History and Downloads independently. This removes the cache and request-
ordering dependency of the initial refresh-based correction. Store, manager,
and browser tests cover the complete path.

## UX-016 — Operational settings require a terminal restart

Status: RESOLVED

Affected:
- app/fetch
- fetch-core
- fetch-server
- web

Expected: Saving operational settings applies them safely without asking users
to stop and restart Fetch from a terminal.

Actual: Download directory, concurrency, bind address, and port were captured
when services were constructed. Only the CIDR allow-list and browser-local
theme changed while Fetch was running.

Resolution: New jobs now read a live default directory and queued jobs use an
adjustable concurrency gate; active jobs are not interrupted. Listener changes
bind and start the replacement address before retiring the old listener, and
the save response moves the browser tab to the new URL. Failed binds roll back
persistence and live download defaults while the original listener stays
available. Browser-open and yt-dlp auto-update preferences remain saved inputs
to their next relevant startup/update action.

## UX-017 — Completed playlists appear as unrelated files

Status: RESOLVED

Affected:
- fetch-core
- fetch-storage
- web

Expected: Completed playlist media remains visibly grouped and ordered without
making standalone downloads harder to browse on desktop or mobile.

Actual: Every completed file used the same large card grid, so playlist context
was lost and collections quickly became difficult to scan.

Resolution: Completed-file responses now derive optional playlist identity,
title, and item index from the persisted download job without a database
migration. The Completed page represents each playlist as a stacked collection
card alongside normal media. Opening it shows only that playlist in the regular
media-card grid, in original order, with URL-backed state and a clear return
action. Realtime insertion uses the same grouping path, and store plus
desktop/mobile browser tests cover collection order and navigation.

## UX-018 — Media playback does not resume across sessions

Status: RESOLVED

Affected:
- fetch-core
- fetch-storage
- fetch-server
- app/fetch
- web

Expected: Fetch remembers meaningful watch positions, communicates progress on
media cards, resumes later playback, and lets the viewer deliberately start
over without introducing user accounts.

Actual: Closing the player discarded its position, so long videos always
restarted and playlist progress was invisible.

Resolution: A cascading SQLite record stores position, actual duration,
completion, and update time per opaque completed-file ID. The player serializes
throttled saves, resumes positions of at least ten seconds, treats the final
five percent as watched, and exposes Start over plus non-blocking save errors.
Cards and playlist summaries render accent progress lines, while dedicated SSE
events synchronize primary, secondary, and LAN views. Domain, storage, API,
store, player, and desktop/mobile browser tests cover the behavior.
