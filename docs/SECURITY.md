# Security Model

Fetch is a local utility, not an internet-facing service.

## Network
Example configuration:

```text
bind_address = 0.0.0.0
port = 8080
allowed_networks = ["192.168.0.0/16"]
```

There is no approved application authentication layer. UI must warn that any reachable device from an allowed network can control downloads and access completed files.

## CIDR
Reject non-allowed remote addresses before API execution. Keep localhost usable unless explicitly disabled. Use a tested CIDR/IP library, not custom bit arithmetic.

## No automatic internet exposure
Never automatically configure router port forwarding, UPnP, public tunnels or internet endpoints.

## Command injection
Never concatenate URL/user input into shell command strings. Use structured process spawning (`Command::new(...).arg(...)`). Avoid `sh -c`, `cmd /C`, PowerShell command strings unless a documented platform requirement safely handles escaping.

## Filesystem
File and thumbnail endpoints use opaque stored IDs, not arbitrary paths.
Validate output directories independently. Playlist titles and IDs are
normalized as single cross-platform path components before directory creation;
raw client text is never appended as a relative path.

Completed-file reveal accepts only an opaque ID and is restricted to clients
whose source address belongs to the host itself. Remote allowed clients can
download and delete completed media but cannot launch host applications. Delete
removes only the persisted media and its cached artwork; it retains download
history. Deleting a completed file also deletes its associated playback
progress through the database relationship.

Playback-progress endpoints accept only opaque completed-file IDs. Progress is
shared application state rather than a private user profile: every client
allowed to use the LAN API can see and change it. This follows Fetch's explicit
no-accounts, no-application-authentication model and must be considered before
granting network access.

LAN QR codes are generated entirely in the embedded frontend from filtered
`/api/network` URLs. Fetch does not send LAN addresses to an external QR service.

System tray actions are local operating-system interactions and are not exposed
as HTTP commands. In particular, LAN clients cannot remotely terminate Fetch.
Changing the per-user system-start registration is also host-only. Registration
uses the current executable path and a fixed `--background` argument; no client
text is inserted into a startup command.

## Static assets
Prevent path traversal and return correct content types.

## State-changing API hardening
Do not use GET for state changes. Validate content types/origin expectations where practical. This is hardening, not authentication.
