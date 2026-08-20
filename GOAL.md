# Active Goal

Develop Fetch 0.1.4 as described in `ROADMAP.md` and `docs/TELEGRAM.md`:

- release the implemented typed JavaScript-runtime controls as part of 0.1.4;
- add an optional Telegram bot that remotely submits and controls downloads
  without exposing the Axum server to the public internet;
- keep the bot disabled by default and authorize only explicitly allowlisted
  Telegram users in private chats;
- store the bot token outside SQLite and never return or retain it in APIs,
  logs, diagnostics, events, or process arguments;
- use outbound long polling rather than webhooks, public ports, tunnels, or an
  additional runtime service;
- reuse Fetch application services for analysis and download operations rather
  than routing bot commands through HTTP or invoking processes directly;
- provide host-only responsive integration settings, actionable connection
  state, bounded retries, persistence, tests, and documentation;
- preserve Fetch's local-first operation when Telegram is disabled or
  unavailable.

Do not upload completed media to Telegram in 0.1.4, introduce user accounts, or
regress the mandatory architecture.
