# Telegram Bot Integration

Status: core implementation complete; 0.1.4 release hardening in progress.

## Purpose

The optional Telegram bot is a remote-control surface for one local Fetch
installation. An authorized user can submit a URL, confirm a video/audio or
playlist download, inspect active work, stop their jobs, and receive terminal
status notifications.

It is not a second Fetch server, a public HTTP endpoint, a Telegram media
library, or an application account system. Fetch continues to run normally when
Telegram is disabled or unreachable.

## Product boundary

0.1.4 supports:

- private chats only;
- an explicit allow-list of numeric Telegram user IDs;
- `/start`, `/help`, `/status`, and `/downloads`;
- URL analysis followed by inline Video, Audio, or Cancel actions;
- a separate confirmation before a complete playlist is queued;
- Stop actions for active jobs created by that Telegram user;
- queued, completed, failed, and stopped notifications;
- host-only configuration and connection diagnostics.

0.1.4 does not support:

- uploading completed media, thumbnails, or cached files to Telegram;
- browsing or deleting the Completed library;
- arbitrary yt-dlp arguments or format IDs;
- accepting commands in groups, channels, or non-allowlisted private chats;
- webhooks, public ports, UPnP, tunnels, or a hosted relay;
- multiple bots or per-user output directories.

## Architecture

```text
Telegram Bot API
       │ outbound HTTPS long polling
       ▼
fetch-telegram adapter
       │ typed updates and requests
       ▼
TelegramBotManager (app/fetch)
       ├── authorization and idempotency
       ├── MediaAnalysis application service
       ├── DownloadOperations application service
       ├── EventBus terminal events
       ├── Telegram repository (fetch-storage)
       └── SecretStore
```

`fetch-telegram` owns Bot API JSON, HTTPS transport, update parsing, message and
callback serialization, timeouts, Telegram error mapping, and redaction. It
does not know about SQLite, Axum, or yt-dlp.

The integration is direct by default. When `use_proxy` is enabled, the adapter
consumes Fetch's shared outbound proxy policy: System mode honors the host proxy
environment, Direct mode disables discovery, and Custom mode uses the validated
HTTP, HTTPS, SOCKS4, or SOCKS5 endpoint. An opted-in active poller watches
policy changes and reconnects without restarting Fetch. Bot API test calls,
polling, replies, and terminal notifications all follow this setting.

`TelegramBotManager` owns polling lifecycle and product flow. It calls the same
core analysis and download services used by the web UI; it must not call Fetch
HTTP routes or execute processes. One cancellation token stops polling during
disable, reconfiguration, or application shutdown.

`fetch-server` exposes only host-authorized integration configuration/status
operations. HTTP handlers call a Telegram application service and never call
Telegram directly.

## Configuration and secrets

Persist non-secret settings separately from normal LAN-readable application
settings:

```text
enabled: boolean
use_proxy: boolean
allowed_user_ids: integer[]
notify_queued: boolean
notify_completed: boolean
notify_failed: boolean
privacy_acknowledged: boolean
```

The Bot API token is write-only. A token entered from the host UI is stored
through a `SecretStore` abstraction backed by the native OS credential store.
The UI can reveal only the unsaved value currently being typed; it never reads
an existing token back from the credential store.
Headless installations may supply `FETCH_TELEGRAM_BOT_TOKEN`; the environment
value takes precedence and cannot be read back or overwritten through the UI.
The settings/status API exposes only `token_configured` and
`token_source = native|environment|missing`.

Never place the token in SQLite, URLs retained by Fetch, tracing fields,
diagnostic details, API responses, SSE payloads, Telegram messages, or command
line arguments. The host-only SSE status event contains non-secret operational
state only. Redact the exact token and Bot API URL token segment from every
transport error before it crosses the adapter boundary.

Only a host browser may read or change Telegram configuration or invoke Test
connection. Remote LAN clients receive `LOCAL_CLIENT_REQUIRED` and the UI does
not request the configuration endpoint. Telegram status events are likewise
filtered out of remote LAN SSE streams.

Planned host-only API surface:

```text
GET  /api/integrations/telegram
PUT  /api/integrations/telegram
PUT  /api/integrations/telegram/token
DELETE /api/integrations/telegram/token
POST /api/integrations/telegram/test
```

The normal settings response contains no Telegram fields. The integration GET
returns non-secret settings and operational status only. Token PUT accepts the
secret over the same-origin host connection and returns no token value.

## Authorization and privacy

The integration is disabled by default. Enabling requires:

- a configured token;
- at least one allowed numeric user ID;
- successful `getMe` validation;
- explicit acknowledgement that submitted URLs, returned titles/status, and
  Telegram account metadata pass through Telegram's service.

Every update is authorized before its text, URL, callback, or command is
processed. Only private chats where `from.id == chat.id` and the user ID is
allowlisted are accepted. Unauthorized updates receive no application data and
cannot create retained message-body or URL logs. Diagnostics may record a
redacted rejection count and a non-reversible identifier fingerprint.

Telegram authorization applies only to the optional bot. It does not add login
or authentication to Fetch's local/LAN web interface.

## Command and callback flow

### URL submission

1. Validate authorization and message shape.
2. Pass the URL to `MediaAnalysis` unchanged.
3. Persist an expiring pending action with an opaque random ID, owner user ID,
   normalized analysis result, and creation time.
4. Reply with title, media/playlist type, duration/count when known, and inline
   **Download video**, **Download audio**, and **Cancel** buttons.
5. On callback, atomically consume the pending action and verify owner and
   expiry before creating jobs.

Callback data contains only an opaque action ID and action code; it never
contains the source URL, token, filesystem path, or yt-dlp arguments. Pending
actions expire after ten minutes and are single-use to prevent replay.

For playlists, Video or Audio opens a second confirmation that states the item
count. Only the explicit confirm callback queues child jobs. The existing
DownloadManager output, concurrency, playlist-folder, and format defaults are
authoritative.

### Jobs and notifications

Persist the Telegram owner and chat correlation for each created job. `/downloads`
shows a bounded list of that user's active jobs with concise progress and Stop
buttons. A Stop callback can affect only a correlated job owned by that user.

Send one queued notice and one terminal completed, failed, or stopped notice
according to saved notification preferences. Do not send a message for every
progress event. Failures use the public error summary and never raw yt-dlp
diagnostics. Completed notices contain title and local completion state, not a
media attachment or a public Fetch URL.

## Polling, reliability, and shutdown

- Use Telegram `getUpdates` long polling; do not register a webhook.
- Persist the last acknowledged update offset transactionally.
- Deduplicate update IDs before any side effect.
- Persist pending actions and job correlations before acknowledging an update.
- Use bounded connect, request, and long-poll durations.
- Honor Telegram retry-after responses and use capped exponential backoff with
  jitter for transient failures.
- Treat invalid/revoked token, conflicting poller, forbidden bot, and malformed
  response as distinct actionable states.
- Hot enabling starts one poller; disabling or token replacement cancels and
  joins the old poller before starting another.
- Application shutdown cancels polling within the existing graceful-shutdown
  bound. Telegram failure never blocks Axum startup or download processing.

## Host UI

Add an **Integrations** settings section that preserves the prototype's card
density and mobile settings navigation. The Telegram card includes:

- Disabled, Connecting, Connected, Backing off, or Error state;
- bot username and last successful poll when available;
- write-only token input with Set/Replace/Remove actions;
- environment-managed token explanation when applicable;
- one numeric allowed user ID per line;
- queued/completed/failed notification toggles;
- privacy acknowledgement;
- Test connection and Save controls with independent progress/errors;
- setup instructions for creating a bot and obtaining the caller's numeric ID.

The token input is always blank after submission. Errors are specific enough to
distinguish missing secret storage, invalid token, network timeout, conflict,
rate limiting, and invalid allow-list values without exposing secrets.

## Implementation checkpoints

1. **Version and domain baseline** — align 0.1.4 metadata; add typed settings,
   status, errors, service/repository traits, and acceptance fixtures.
2. **Persistence and secrets** — add migrations, idempotency transactions,
   native secret storage, headless environment override, and redaction tests.
3. **Transport and lifecycle** — add `fetch-telegram`, long polling, Bot API
   actions, backoff/rate limits, manager hot reconfiguration, and shutdown.
4. **Command vertical slice** — implement authorization, help/status, URL
   analysis, confirmations, queue/Stop, correlations, and notifications.
5. **Host API and UI** — add host-only contracts, responsive Integrations UI,
   status refresh/events, validation, and setup guidance.
6. **Hardening and release** — complete deterministic integration/browser
   tests, opt-in live smoke, security review, docs, packaging, and release gates.

Each checkpoint is committed only after its focused tests pass. No checkpoint
may put a real bot token, chat transcript, external URL, database, or generated
test artifact into Git.

## Acceptance criteria

- [x] Telegram is disabled by default and Fetch works normally without a token
  or Telegram network access.
- [x] Enabling, disabling, token replacement, and shutdown leave at most one
  polling task and never interrupt HTTP/download services.
- [x] Tokens never enter SQLite or observable API/log/event/process surfaces.
- [x] Only allowlisted private-chat users can read state or create/control jobs.
- [x] Duplicate updates and replayed/expired callbacks cannot duplicate jobs.
- [x] URL analysis and download creation reuse core services without HTTP or
  direct process execution.
- [x] Playlist downloads require a count-bearing second confirmation.
- [x] Notifications are bounded, preference-aware, and contain no media files,
  raw diagnostics, filesystem paths, or Fetch LAN URLs.
- [x] Host-only settings are usable on desktop and mobile; LAN browsers cannot
  request integration secrets or configuration.
- [x] Timeouts, rate limits, invalid tokens, poller conflicts, network loss, and
  recovery produce actionable redacted state.
- [x] Connection attempts, results, retry delays, and proxy-route reconnects
  are retained with proxy mode only; endpoints and tokens are never logged.
- [x] Telegram is direct by default; when proxy use is enabled, shared policy
  changes reconnect the active long poll. Tests finish before the UI deadline.
- [x] Deterministic tests require no Telegram account; live smoke is opt-in and
  secret-backed.
- [ ] Full local and native release gates pass with documentation aligned.

## Deferred

- group/channel operation;
- multiple bot profiles;
- uploading or streaming completed files through Telegram;
- browsing/deleting the Completed library;
- per-user formats, output roots, history, quotas, or recommendations;
- webhook deployment or a hosted Fetch relay.
