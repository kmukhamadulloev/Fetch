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
