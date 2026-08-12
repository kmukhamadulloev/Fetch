# Active Goal

Stabilize Fetch 0.1.3 after the proxy implementation described in `ROADMAP.md`
and `docs/PROXY.md`:

- add typed, persisted, host-only outbound proxy configuration;
- hot-apply the selected route to yt-dlp analysis and newly spawned downloads;
- integrate responsive Network settings without exposing the endpoint to LAN
  clients or retained diagnostics;
- validate the Rust, API, frontend, browser, and compatibility paths required
  by the proxy acceptance criteria;
- keep `ROADMAP.md`, `RELEASE.md`, `docs/ACCEPTANCE.md`, and `ISSUES.md` aligned
  with verified behavior.
- preserve an already healthy managed runtime when an automatic update fails;
- bound managed-runtime network stalls and retain actionable lifecycle details;
- add responsive severity filters to the retained Logs interface.

Do not regress the mandatory architecture or introduce a forbidden component.
