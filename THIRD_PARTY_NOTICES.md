# Third-party notices

Fetch is distributed under the MIT License. Its Rust and frontend dependency
graphs are recorded exactly in `Cargo.lock` and `web/package-lock.json`.

The embedded frontend uses `qrcode.vue`, distributed under the MIT License, to
render LAN access QR codes entirely in the browser. See
<https://github.com/scopewu/qrcode.vue>.

Native lifecycle integration uses `tray-icon` (MIT OR Apache-2.0) on Windows
and macOS and `ksni` (Unlicense) on Linux. Exact versions are recorded in
`Cargo.lock`; per-user startup files/registry values use operating-system APIs.

Fetch does not bundle yt-dlp, FFmpeg, or FFprobe in its release archive. At
runtime it can download them into the local application-data directory:

- yt-dlp is obtained from the official `yt-dlp/yt-dlp` GitHub release. The
  project is offered under the Unlicense, and standalone release files can
  contain separately licensed components. See
  <https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE> and
  <https://github.com/yt-dlp/yt-dlp/blob/master/THIRD_PARTY_LICENSES.txt>.
- FFmpeg and FFprobe are obtained from the `eugeneware/ffmpeg-static` release
  provider. The provider is GPL-3.0-or-later and republishes builds from the
  platform providers described at
  <https://github.com/eugeneware/ffmpeg-static>. Fetch downloads the matching
  provider license beside the binaries as `FFMPEG-LICENSE.txt`, and records
  the provider and release in `PROVIDER.txt`.
- FFmpeg itself is primarily LGPL-2.1-or-later, while builds that enable GPL
  components are GPL-2.0-or-later. The downloaded build's accompanying license
  controls. See <https://ffmpeg.org/legal.html>.

These programs are separate executables and are not linked into Fetch. Users
remain responsible for complying with applicable media-service terms,
copyright law, and the licenses of downloaded runtime components.
