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
File endpoints use opaque stored IDs, not arbitrary paths. Validate output directories independently.

## Static assets
Prevent path traversal and return correct content types.

## State-changing API hardening
Do not use GET for state changes. Validate content types/origin expectations where practical. This is hardening, not authentication.
