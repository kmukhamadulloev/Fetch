# Editor playback fixture

`editor-source.webm` is synthetic local test-pattern video (96×54, 10 fps,
12 seconds, VP8, no audio), used only by Playwright to exercise real browser
playback, selection and crop interactions. It is not bundled by Vite.

Regenerate with:

```sh
ffmpeg -f lavfi -i testsrc2=size=96x54:rate=10:duration=12 -an -c:v libvpx -b:v 30k editor-source.webm
```
