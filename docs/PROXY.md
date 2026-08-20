# Proxy Support Design

Status: implemented for yt-dlp in Fetch 0.1.3 and extended to Telegram in
Fetch 0.1.4.

## Scope

Fetch will provide one host-managed outbound route for yt-dlp operations:

- `system`: do not add an explicit proxy argument, allowing the process
  environment and yt-dlp defaults to apply;
- `direct`: pass an explicit empty `--proxy` value so yt-dlp connects directly;
- `custom`: pass a validated HTTP, HTTPS, SOCKS4, or SOCKS5 proxy URL.

The selected route applies to media and playlist analysis and each yt-dlp
download process when it is spawned. Telegram Bot API traffic can opt into it
with its integration setting and is explicitly direct otherwise. Changing the
route does not interrupt an active download; queued work uses the latest
setting. An opted-in Telegram poller cancels its current request and reconnects
immediately through the new route.

The proxy does not route Fetch's Axum listener, browser-to-Fetch traffic, file
streaming over the LAN, browser navigation, or managed runtime downloads.

## Security boundary

The first implementation accepts unauthenticated proxies only. Reject URLs
containing a username or password. This avoids exposing credentials through
process arguments, process listings, SQLite, API responses, or yt-dlp output.

Only a request originating from the Fetch host may read or change proxy
configuration. Allowed LAN clients can still analyze and queue media, and that
work uses the host's selected route, but they cannot discover or alter the
proxy endpoint. Fetch continues to provide no application authentication.

Never build a shell command. Append `--proxy` and its value as separate process
arguments. Treat the endpoint as sensitive operational data: do not include it
in normal logs, persisted diagnostics, SSE events, or public status responses.
Sanitize subprocess failures before persistence or API mapping.

## Domain and persistence

Introduce a typed configuration independent of `DownloadRequest`:

```text
ProxySettings
├── mode: system | direct | custom
└── url: optional validated proxy URL
```

Persist it separately from `ApplicationSettings`, whose GET response is
available to allowed LAN clients. Existing databases default to `system`.
Downloads reference the active policy at process-spawn time rather than
copying the endpoint into job JSON or download history.

A shared proxy-policy provider is composed into the yt-dlp and Telegram
adapters. The
host-only application service validates and persists a replacement before
publishing it to that provider. Failed persistence leaves the active policy
unchanged.

## Validation

For custom mode:

- parse with a URL parser rather than string matching;
- allow only `http`, `https`, `socks4`, and `socks5` schemes;
- require a valid host and a usable explicit or scheme-default port;
- reject user information, query strings, fragments, and non-root paths;
- enforce a reasonable maximum serialized length;
- accept hostnames, IPv4, and bracketed IPv6 addresses.

System and direct modes require no URL. Reject a URL supplied for either mode
so the saved state remains unambiguous. Validation does not promise that an
endpoint is reachable.

## API contract

Add host-only endpoints:

```text
GET /api/proxy
PUT /api/proxy
```

The PUT body and successful response use the typed proxy settings shape. A
non-host request receives `LOCAL_CLIENT_REQUIRED`. Invalid mode/URL
combinations receive `INVALID_SETTINGS`. Proxy connection failures during
analysis or download return an actionable typed summary without the endpoint.

Do not add arbitrary yt-dlp arguments or accept a proxy override in a download
request in this version.

## User interface

Add an **Outbound connections** card to Settings → Network:

- a mode selector for System default, Direct connection, and Custom proxy;
- a proxy URL input shown only for Custom proxy;
- supported-scheme and unauthenticated-only guidance;
- saved/error feedback consistent with existing settings;
- explanatory text that active downloads are not restarted and opted-in
  Telegram reconnects;
- a disabled host-only state on LAN devices.

Do not imply that the proxy protects or exposes the Fetch web interface. Do
not display the configured endpoint to a LAN client.

## Acceptance criteria

- [x] PASS — `system`, `direct`, and `custom` produce the intended deterministic yt-dlp
  arguments for both analysis and downloads.
- [x] PASS — Valid HTTP/HTTPS/SOCKS4/SOCKS5 endpoints persist and hot-apply.
- [x] PASS — Invalid schemes, malformed URLs, credentials, and ambiguous mode/URL
  combinations are rejected without changing the active setting.
- [x] PASS — Existing databases load with `system` mode and require no manual migration.
- [x] PASS — LAN clients cannot read or modify proxy configuration.
- [x] PASS — The proxy URL never appears in download job JSON, SSE payloads, normal logs,
  retained diagnostics, or API errors.
- [x] PASS — Changing the setting affects queued/new processes and does not terminate an
  active download.
- [x] PASS — Telegram is direct by default; when opted in, tests, polling,
  replies, and notifications use the shared route and reconnect on changes.
- [x] PASS — Desktop and mobile UI clearly represent all modes, validation failures, and
  the LAN-disabled state.
- [x] PASS — The local server, LAN file transfer, and managed runtime installation remain
  unaffected.

## Deferred

- authenticated proxies and native secret storage;
- named proxy profiles;
- per-download overrides;
- automatic proxy-to-direct fallback;
- proxy routing for managed runtime installation and updates;
- external proxy-health or IP-check services.
