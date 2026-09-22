# Active Goal

Implement the approved completed-media metadata editor for Fetch.

- Add a rounded pencil button at the top-left of each individual card image,
  including placeholder images; clicking opens a responsive metadata modal.
- Basic audio fields: title, cover artwork, artist, album, album artist, track
  number. Basic video fields: title, cover artwork, creator, description.
- A collapsed Advanced accordion exposes supported release date, genre,
  copyright, comment, audio description, total tracks, disc number, total discs,
  and composer fields. Do not duplicate tags with the same underlying meaning.
- Cover preview, upload/replace, and removal are required in this first scope.
  Save artwork into the file and synchronize the library thumbnail.
- Show read-only filename, container, size, duration, codecs, bitrate, video
  resolution/frame rate, and audio sample rate/channels separately.
- Read actual embedded tags with managed FFprobe; save supported tags/artwork
  with managed FFmpeg without re-encoding audio/video or renaming files.
- Initially support tested MP3, M4A, FLAC, MP4, and MKV combinations; expose
  unsupported files read-only with an explanation rather than false success.
- Preserve other streams, chapters, artwork, and metadata; reject unsafe edits.
  Use validated temporary outputs, conflict protection, recoverable replacement,
  SQLite synchronization, background saves, and SSE library refresh.
- Preserve allowed-LAN access, opaque file IDs, the Rust service/adapter boundary,
  Vue styling, and English/Russian/Tajik localization.
- Provide Save/Cancel, truthful status/errors, keyboard accessibility, and an
  unsaved-changes guard.

Checkpoints and acceptance criteria are in `docs/METADATA.md` and `ROADMAP.md`.
Release numbering, tagging, and publication are outside this implementation scope.

Implementation and local verification are complete. Evidence is recorded in
`docs/ACCEPTANCE.md`. Native Windows/macOS execution and release publication
remain separate release gates.
