# Local media processing

Status: implementation contract for the active unversioned goal. This document
separates planned behavior from delivered acceptance evidence. See GOAL.md.

Delivered foundation: typed validated requests/lifecycle, migration 0008,
persistent process records with private recovery options, nullable download
linkage and historical source/origin fields, and an atomic database transaction
for output registration plus process completion. Scheduler, runtime adapter,
HTTP processing routes, and new UI are still pending; checkpoint 2 is incomplete.

## Supported output matrix

| Output | Re-encode | Quality presets | Stream copy |
| --- | --- | --- | --- |
| MP4 | H.264 (`libx264`), AAC | compact CRF 28 / balanced 23 / high 18; AAC 128/192/256 kb/s | H.264 + optional AAC |
| MKV | H.264 (`libx264`), AAC | same as MP4 | H.264 + optional AAC |
| M4A | AAC | 128/192/256 kb/s | AAC |
| MP3 | `libmp3lame` | 128/192/320 kb/s | MP3 |
| FLAC | `flac` | lossless; no lossy quality control | FLAC |
| WAV | `pcm_s16le` | 16-bit PCM; no lossy quality control | unavailable |

Audio outputs accept audio-only sources or extract audio from video. Lossless
output does not restore previously lost detail. Quick edits use these same
outputs and always re-encode. Prefer the source extension when supported;
otherwise require an explicit supported output choice. Query installed managed
FFmpeg encoders before offering operations; recheck when executing a queued job.
No arbitrary codec, protocol, path, or argument input is accepted.

Linux managed-runtime capability probe (2026-09-23): generated one-second
64x48 H.264/AAC media and exported all six formats plus MP4/MKV stream copy;
FFprobe confirmed expected codecs for all eight outputs. This is preliminary
matrix evidence, not completion of application integration or native testing.
The final opt-in adapter tests must reproduce this with the application adapter.

## Streams and preservation

Select the first non-artwork video and first audio stream. The capability
response reports extra streams, artwork, chapters, and tags. Preserve format
tags supported by the destination; retain chapters only on full-length MP4/MKV
exports. Copy compatible attached artwork where the destination supports it.
Omissions (additional tracks, subtitles, attachments, unsupported artwork/tags,
trimmed chapters) require explicit acknowledgement in the form and request.
Do not silently advertise universal preservation. The adapter must verify the
selected streams and expected duration/dimensions before publishing.

## Quick-edit validation

Times are finite seconds, start >= 0, end > start, within the probed duration.
Trim after decoding, with output duration bounded to end minus start; allow one
video frame plus audio encoder delay in tests. Reset timestamps consistently
for audio and video. Stream copy is never offered for edits.

Apply display orientation, then crop, then requested rotation (0/90/180/270),
then resize. Remove obsolete rotation metadata. Crops are integer rectangles
inside the display-oriented source; output H.264 dimensions must be positive
even integers, at most 7680 per axis. Resize defaults to source dimensions and
must preserve aspect ratio unless the user explicitly chooses otherwise.
Mute removes audio; volume is finite 0..4 and unavailable with mute or no audio.
Audio-only media offers trim/volume, not crop/rotate/resize/mute. Reject exports
with no selected stream or no requested edit. Combine filters into one pass.

## Lifecycle and concurrency

Persistent kinds: conversion, edit, metadata. States: queued, running,
completed, failed, cancelled, interrupted. Stages identify inspection,
processing, validation, publication. Percent and ETA are nullable; parse actual
FFmpeg `-progress` output, cap processing progress below 100 until publication.
Persist timestamps and terminal errors; expose sanitized errors, not local paths.

A single application-owned FIFO scheduler executes these jobs. Metadata's
existing replacement journal stays authoritative. Downloads retain their own
manager and concurrency. The scheduler shares the existing metadata/library
mutation mutex: running exports block deletion/replacement. Queued exports
capture source revision and output directory; validate the revision at execution
and again before publication. A missing/changed source fails without export.

Exports: cancel queued/running work, kill and reap the child, remove temporary
files. Retry failed/cancelled/interrupted work as a new attempt from the start,
with new identity and current validated revision; never pretend to resume.
Metadata: no cancel during replacement; retain existing Saving/Completed/Failed
API semantics, with queued state represented as Saving to legacy callers.
Shutdown interrupts running exports; queued work survives restart. Recover
metadata journals before accepting new work. No browser connection owns a job.

## Persistence and crash recovery

Migration adds persistent processing jobs and optional source/origin fields to
completed files. Download job linkage becomes nullable; existing download IDs,
playlist context, playback records, thumbnails, and metadata journals survive.
Derived entries have no download job and no playlist, fresh playback, and a
source UUID that remains historical if its source is deleted. Test upgrading a
populated pre-migration database with foreign keys enabled, not just an empty DB.

Resolve the configured root at acceptance; lazy-create Converted/ or Edits/.
Validate safe basename and reserve output without overwriting, including races.
Work in a unique sibling temporary file. Persist output/publication intent,
FFprobe-validate and fsync the temporary file, publish using a no-clobber primitive,
then atomically insert the completed row and mark the job completed in SQLite.
Startup reconciles the intent: either finish the validated publication or remove
an uncommitted output owned by that job. Never delete unrelated collision files.
A successful derived output remains when its source is deleted.

## API and events

Planned routes, all under the existing allowed-client policy:

- GET `/api/files/{id}/processing`: source revision/technical data, supported
  outputs/modes/quality, edit limits, preservation notices, and runtime readiness.
- POST `/api/files/{id}/processes`: typed conversion/edit request, revision,
  basename, format, quality, copy flag, edit options, omission acknowledgement;
  returns the accepted process record. Reject unknown fields.
- GET `/api/processes`: persisted non-download processes newest first.
- POST `/api/processes/{id}/cancel` and `/retry`: enforce state/kind actions.
- Existing metadata routes retain their contract, also publishing process state.

Public records contain opaque operation/source/output UUIDs, kind, title, state,
stage, nullable percent/ETA, timestamps and sanitized error; no filesystem paths.
SSE `process.updated` carries the same record. Processes UI merges this snapshot
with existing downloads without creating duplicate download jobs. Refresh both
snapshots after reconnect in primary and secondary tabs. `/downloads` redirects
to `/processes`; a process query parameter focuses a row from submission feedback.

## Checkpoint tests

1. Contract: managed codec probe above and documentation-link checks.
2. Foundation: populated migration, scheduler serialization including metadata,
   restart/recovery at publication boundaries, no-clobber names, cancellation,
   malformed requests, source revision/deletion, missing runtime, safe argv.
3. Processes: API access/state actions, snapshot/SSE/reconnect, download regressions,
   filters and old-route compatibility; persist history across restart.
4. Convert: actual eight-output matrix, extraction/copy hash comparison, tags and
   omissions, separate library entries/root snapshot; modal/menu accessibility.
5. Edit: actual trim/rotate/mute combinations, A/V sync, original hash unchanged.
6. Extended edits: crop/resize/volume alone and combined, invalid geometry/ranges,
   orientation and unsupported-preview behavior.
7. Full repository suite, production desktop/mobile browser suite, opt-in real
   runtime tests, dirty guards/focus/three locales, and individual A1–A10 evidence.
