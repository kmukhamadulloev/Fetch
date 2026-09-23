# Active Goal

Implement separate **Convert** and **Quick edit** workflows for completed media,
and evolve **Downloads** into **Processes** so users can follow downloads,
conversions, edits, and metadata saves in one place.

Status: in progress; contracts and capability design verified. This goal is unversioned.
Release numbering, tagging, pushing, and publication are separate work.

## Read first

Follow [AGENTS.md](AGENTS.md). Read [architecture](docs/ARCHITECTURE.md),
[download engine](docs/DOWNLOAD_ENGINE.md), [metadata editing](docs/METADATA.md),
[runtime management](docs/RUNTIME.md), [UI specification](docs/UI_SPEC.md),
[API](docs/API.md), [security](docs/SECURITY.md), [testing](docs/TESTING.md),
[ROADMAP.md](ROADMAP.md), and [ISSUES.md](ISSUES.md) before implementation.

The existing subsystem docs describe delivered behavior. This goal approves the
new card menu, Processes navigation, and explicit local FFmpeg export operations;
update subsystem contracts and docs as each checkpoint is implemented.

## Approved scope and boundaries

- Keep the mandatory Rust/Axum/Tokio, Vue/TypeScript/Tailwind/Pinia, SQLite, SSE,
  and embedded-frontend architecture. Use managed FFmpeg/FFprobe binaries.
- This is an explicit exception to the old FFmpeg post-processing-only scope:
  user-requested conversion and quick edits may re-encode completed local files
  into saved outputs. Fetch remains a local-first downloader with media tools.
- Inputs are existing completed-library files resolved by opaque IDs. Preserve
  the current allowed-LAN access model and host-only settings boundaries.
- No arbitrary-file import, browser uploads of source media, arbitrary FFmpeg
  arguments, new authentication, cloud processing, or external service.
- No live transcoding, HLS, automatic playback conversion, or transcoding preview
  service. Play compatible source files with the existing Range-backed player;
  explain unavailable preview and keep Open/Download available otherwise.
- No multi-clip/multi-track timeline, transitions, effects suite, batch processing,
  or GPU encoding in this goal.

## User experience

### Completed-card menu

Replace the top-left pencil on each individual completed-media card, including
placeholder images and cards inside a playlist, with a vertical three-dot menu:

1. **Edit metadata** — opens the existing metadata modal.
2. **Convert** — opens the separate conversion form.
3. **Quick edit** — opens the separate media-editing modal.

Keep the location, touch-target size, spacing, dark application character, and
existing playback/file actions. Do not add these actions to playlist collection
cards. Opening or using the menu must not trigger playback. Support keyboard
navigation, accessible names, Escape/outside-click dismissal, and focus return.

### Convert modal

A compact responsive form provides:

- source title and read-only technical information;
- video/audio output type, supported target format, and simple quality presets;
- audio extraction for video sources and audio conversion for audio sources;
- safe output filename and the `Converted/` destination;
- a clear distinction between stream copy and re-encoding, with any relevant
  stream/metadata limitations explained before submission;
- **Cancel** and **Start conversion** in the footer.

Offer a tested allowlist backed by capabilities of the installed managed FFmpeg,
not every extension FFmpeg recognizes. Define and document the exact supported
container/codec/preset matrix in checkpoint 1. Cover common video conversion,
compatible container changes, audio conversion, and audio extraction; never
claim that changing the extension alone converts media or improves lost quality.

### Quick edit modal

A wider responsive modal provides source preview when browser-compatible,
start/end time controls, relevant editing options, output filename, the `Edits/`
destination, and **Cancel** / **Export edited copy** actions.

Deliver quick edits incrementally: trim, rotate, and mute first; then crop,
resize, and volume adjustment. All six belong to this goal. Show controls only
where applicable to the source media. Combine selected operations into one
export job, avoiding successive lossy conversions. Preserve the source format
where supported; explain and offer a supported output when that is not possible.
Specify trim accuracy, valid ranges, dimensions, rotation/orientation handling,
and audio/video synchronization. Do not present keyframe-limited copying as
frame-accurate trimming. Do not fabricate a preview for unplayable sources.

### Shared modal behavior

- Match the existing metadata modal's styling and accessibility. Keep the three
  tools separate rather than building a combined editor/converter form.
- On mobile, use nearly full-screen modals with scrolling content and a fixed,
  visible action footer. Trap focus, restore it on close, and guard dirty dismissal.
- Disable duplicate submissions. Close only after the backend accepts the job;
  show **Added to Processes** with a **View process** action that focuses that job.
- Stay on the current page unless the user chooses View process. Submission
  failures keep the modal open with all inputs intact and an actionable error.
- Keep metadata-only Save semantics: update the existing file safely using the
  existing recovery rules. Conversion/edit export always creates a new file.
- Translate all new UI, statuses, errors, and accessibility text into English,
  Russian, and Tajik.

## Files and library behavior

- Create `Edits/` and `Converted/` inside the configured download directory, not
  beside the executable or in the application's private data directory.
- Create each folder lazily. Generate safe unique filenames; never overwrite
  originals or an existing output, including under concurrent requests.
- Resolve and retain each job's output root when queued. A later setting change
  applies to new jobs and does not move existing files or active outputs.
- Successful exports become independent Completed entries with **Edited** or
  **Converted** origin labels and a persisted reference to their source.
- Preserve original IDs, download history, playlist order, and playback progress.
  Derived files receive new IDs and fresh playback progress; do not silently
  insert them into the original playlist or manufacture download jobs for them.
- Preserve supported tags, artwork, chapters, and selected streams. Make intended
  omissions or unsupported preservation explicit; do not silently discard them.
- Validate temporary outputs with FFprobe before publishing the file and library
  record. Failure/cancellation must preserve the source and clean partial output.
- Coordinate exports with source deletion and metadata replacement. Detect a
  missing or changed queued source. Define recoverable filesystem/SQLite commit
  ordering; retain derived files if their source is subsequently deleted.

## Processes and backend responsibilities

- Rename the Downloads navigation entry and page to **Processes**, preserving
  the current desktop/mobile navigation and list density. Use `/processes` and
  keep `/downloads` working as a redirect or alias for existing links.
- Add type filters: All, Downloads, Conversions, Edits, Metadata updates, alongside
  existing status filters. Keep the current download behavior and controls.
- Rows show title, operation/type, real stage/progress, elapsed time, ETA only
  when known, actionable errors, and a link to the resulting file where applicable.
- Download Stop/Resume/Retry keep their existing semantics. Conversion/edit jobs
  support Cancel and retry from the beginning; never advertise partial resume.
  Metadata saves show an indeterminate indicator when percentage is unavailable
  and expose only actions supported by their safe replacement lifecycle.
- Use one shared application-owned FFmpeg job scheduler for metadata saves,
  conversions, and edits, initially with one such operation active at a time.
  Downloads remain managed by DownloadManager with their existing concurrency;
  the common Processes view must not replace or duplicate download execution.
- Persist operation identity, type, source/output linkage, requested options,
  lifecycle timestamps, state, and failure details in SQLite. Restore queued
  work safely and mark interrupted running exports truthfully after restart.
  Preserve existing metadata replacement recovery and completed-file schemas
  through tested backward-compatible migrations.
- Keep routes thin: HTTP -> core/application service -> manager/repository ->
  FFmpeg/FFprobe adapter. Never execute processes in HTTP handlers or frontend code.
- Use structured allowlisted arguments and validated local input/output paths.
  Report missing encoders/runtimes, unsupported media, disk errors, cancellation,
  invalid edits, and process failures without fake success or leaked private paths.
- Parse actual FFmpeg progress; publish typed SSE state and library events. Load
  persisted snapshots on refresh/reconnect so primary, secondary, and LAN tabs
  converge. Treat unknown progress/ETA as unknown.

## Ordered implementation checkpoints

Complete these in order. Each checkpoint must leave working, reviewable behavior
and pass its relevant checks before the next one begins.

1. **Contracts and capability design:** inspect existing managers, persistence,
   card/modal components, and routes. Document the tested target matrix, edit
   validation, lifecycle/actions, source conflict rules, recovery, API/SSE shapes,
   and schema compatibility. Define focused tests before building each slice.
2. **Persistent processing foundation:** implement typed domain contracts,
   migrations, the shared FFmpeg scheduler, real managed-runtime adapter, progress,
   cancellation, output safety, and crash recovery. Integrate existing metadata
   saves without losing their preservation, conflict, and recovery guarantees.
3. **Processes vertical slice:** expose persisted process listing/control and
   SSE; integrate real downloads and metadata updates in the renamed page, with
   type/status filters, valid actions, history, and old-route compatibility.
4. **Conversion vertical slice:** implement the supported conversion/extraction
   operations, capability validation, `Converted/` outputs, library registration,
   card menu, and separate Convert modal. Keep Edit metadata fully functional;
   do not expose a placeholder Quick edit action before its backend is ready.
5. **Quick edit vertical slice:** implement trim/rotate/mute through the same
   scheduler with `Edits/` outputs, preview/controls, and the separate modal. Add
   the functional Quick edit menu action; verify combined edits and A/V sync.
6. **Remaining quick edits:** add crop/resize/volume controls and validated
   combinations, retain originals and metadata, and handle unsupported previews
   or outputs honestly. Do not mark the whole goal complete after checkpoint 5.
7. **Hardening and evidence:** finish error/recovery/accessibility/localization
   coverage, real managed-FFmpeg smoke tests, desktop/mobile review, and docs.
   Run the full required suite and record acceptance evidence individually.

After each checkpoint, format/lint, run relevant unit/integration/frontend/browser
checks and builds, update the roadmap and subsystem/API docs, and record any real
issues. Create one focused local commit per verified checkpoint following the
repository workflow. Keep unfinished checkpoints unchecked; do not claim passing
acceptance based only on compilation or mocked process output.

## Acceptance criteria

All items below are pending until verified and recorded in `docs/ACCEPTANCE.md`.

- A1: Three-dot menu opens the correct separate tool without triggering playback;
  metadata editing retains all previously accepted behavior.
- A2: Convert handles the documented container/codec matrix and extraction using
  actual managed tools; supported stream copy avoids re-encoding.
- A3: All six quick edits work individually and in supported combinations, with
  accurate duration/orientation/dimensions, correct audio behavior, and A/V sync.
- A4: Exports preserve originals and create collision-safe files in the correct
  folder, independent library entries, origin labels, and source linkage.
- A5: Processes displays downloads, conversions, edits, and metadata updates with
  truthful state/progress, working filters/actions, history, and route compatibility.
- A6: Cancellation, retry, missing/changed sources, disk/runtime/encoder failures,
  concurrent requests, and restart recovery leave consistent files and SQLite data.
- A7: Initial load/reconnect and primary/secondary/LAN views remain synchronized;
  local-file access stays opaque-ID based and existing access boundaries hold.
- A8: Desktop/mobile forms, dirty guards, keyboard/focus behavior, submission
  feedback, and English/Russian/Tajik translations pass browser checks.
- A9: Tests exercise real managed FFmpeg/FFprobe with synthetic local media in
  addition to deterministic unit/integration fixtures; docs/contracts match code.
- A10: `FETCH_RUN_E2E=1 ./scripts/check.sh`, the documented real-runtime smoke
  commands, and `git diff --check` pass. Report native-platform verification and
  release publication separately; do not represent unexecuted gates as passed.
