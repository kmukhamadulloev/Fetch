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
