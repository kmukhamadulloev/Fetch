# Phase 05 — Completed Files and Simple Playback

## Objective
Expose completed files without building a transcoding server.

## Required work
- Completed API;
- opaque file IDs;
- file download endpoint;
- Range stream endpoint;
- MIME detection;
- safe path handling;
- Completed UI;
- native browser playback where supported;
- Download fallback otherwise.

## Acceptance criteria
- [ ] arbitrary filesystem paths cannot be requested.
- [ ] completed list works.
- [ ] download endpoint works.
- [ ] valid Range requests return correct 206.
- [ ] seeking works for compatible media.
- [ ] unsupported media is not transcoded.
- [ ] UI falls back to Download.
- [ ] tests pass.
