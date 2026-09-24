# Test Fixtures

Fake yt-dlp fixtures are allowed here only for deterministic automated tests.

They MUST NOT:
- ship in release artifacts;
- be selected by production configuration;
- replace real yt-dlp smoke testing.

Runtime health tests symlink the checked-in executable `fake-ytdlp.sh` and
`fake-node.sh` into temporary directories. Keep these fixtures immutable during
tests: writing executable scripts alongside concurrent process spawning can
cause intermittent Linux `ETXTBSY` failures.
