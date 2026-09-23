# Completed-media metadata editor

## Approved scope

The top-left pencil on an individual completed card opens a modal. Basic audio
fields are title, artwork, artist, album, album artist, and track number. Basic
video fields are title, artwork, creator, and description. Advanced starts
collapsed and contains supported date, genre, copyright, comment, audio
description, total tracks, disc number, total discs, and composer fields.
Technical file information is read-only. Title changes never rename files.

Artwork preview, replacement with a local JPEG/PNG, and removal are required.
Only supported container/tag combinations are editable. MP3, M4A, FLAC, MP4,
and MKV are the initial targets; unsupported files explain their limitation.

## Engineering checkpoints

1. Document approved scope and acceptance criteria.
2. Implement typed metadata contracts, managed FFprobe/FFmpeg adapter, safe
   background save service, recovery persistence, API and runtime tests.
3. Implement the card action and localized accessible modal, then synchronize
   API/UI/testing documentation and run repository validation.

## Acceptance criteria

- A1: Individual audio/video cards open the modal through an accessible rounded
  top-left pencil, without triggering playback; playlist cards remain unchanged.
- A2: Basic/Advanced fields match the approved audio/video split and actual
  container capabilities; technical information is read-only.
- A3: Real embedded tags can be read, changed, and cleared without renaming or
  re-encoding media; preserve unrelated streams, chapters, and supported tags.
- A4: Artwork can be previewed, replaced, and removed in supported formats;
  embedded artwork and cached thumbnail agree after changes.
- A5: Background saves expose truthful status/errors, reject stale/concurrent
  edits, protect originals on failure, and recover interrupted replacement.
- A6: SQLite title/size/artwork and connected libraries refresh after saving;
  playback progress, IDs, playlist order, and download history remain intact.
- A7: Modal supports keyboard focus, Escape/Cancel and dirty-state guarding,
  narrow screens, and complete English/Russian/Tajik localization.
- A8: Rust format/lint/unit/integration checks and frontend lint/typecheck/unit/
  build/browser checks pass; API and behavior documentation match implementation.

## Non-goals

Batch edits, arbitrary-file imports, filename templates, chapter editing,
online tag lookup, transcoding, and new authentication are outside this scope.

## Implemented design

`CompletedLibrary` delegates to an application `MetadataService`; its managed
process adapter owns FFprobe/FFmpeg invocation. HTTP accepts opaque file IDs and
allowlisted field names, never paths or process arguments. Allowed LAN clients
have the same editing access as existing completed-file deletion.

Metadata saves enter the persistent processing queue shared with conversions
and quick edits; one operation runs at a time. Queued saves retain the legacy
`Saving` status. Process history and metadata status survive restart. Deletion and other metadata inspections
receive a conflict while saving; existing file streams are not cancelled. A
revision derived from file size and nanosecond modification time rejects stale
edits before processing and immediately before replacement. This detects normal
external modifications, not an adversarial rewrite preserving both attributes.

The service persists a SQLite recovery journal before writing a sibling output.
It validates tag round trips, stream signatures, artwork presence, unrelated
container tags, and chapters before replacement. FFmpeg uses stream copy for
all media streams. Original file permissions are retained. Outputs are flushed,
the original is renamed to a unique backup, and the new file takes its place.
SQLite updates the library and durable commit marker in one transaction.
Recovery restores an uncommitted backup or cleans a committed operation; it runs
before serving on startup. Directory changes are flushed on Unix. Windows
open-file rename failures are surfaced and the journal retains recovery data.
No filename, completed ID, job history, playlist context, or playback progress
is changed. Save time and temporary space scale with file size.

The adapter bounds subprocess duration and captured output, restricts input
protocols to local files, and kills child processes on cancellation. Submitted
artwork is limited to 8 MiB JPEG/PNG, 8192 pixels per side, and 128 MiB decoded
allocation. It is normalized to an RGB JPEG up to 2048 pixels per side; those
same bytes become the library thumbnail. Removing artwork clears both embedded
artwork and the cached thumbnail. Keeping artwork preserves existing thumbnail
behavior; when only a cached thumbnail exists the modal labels that distinction.
New cover files use private `.fetch-<uuid>.jpg` sibling names managed by the
completed-file lifecycle. HTTP thumbnail responses revalidate and library
updates also change the thumbnail URL in mounted cards.

MP3, M4A, FLAC, MP4, and MKV tags/artwork are tested with actual managed tools.
FLAC exposes Comment only because its muxer aliases Description/Comment. MKV
cover images must be extracted without re-encoding and reattached; otherwise
FFmpeg can turn an attached image into a timed video stream. Files with multiple
MKV covers are rejected when keeping covers. Unusual streams or tags that a
muxer cannot preserve cause a save failure with the original retained.

The form sends only changed tags. Track and disc totals are combined with their
numbers for embedded tags; a total requires a number and cannot be smaller.
Date is a text field so users may retain a year or fuller date. Technical
information is read-only. The modal traps keyboard focus, restores focus to the
pencil, locks background scrolling, guards route/tab dismissal, and keeps its
footer visible on phones. Save status uses SSE plus a bounded REST polling
fallback; no percentage is fabricated. Latest operation status is session-local
(bounded to 256 entries); after restart clients reopen the recovered file.

FFmpeg behavior references: [stream copy and metadata](https://ffmpeg.org/ffmpeg.html)
and [container formats](https://ffmpeg.org/ffmpeg-formats.html).
