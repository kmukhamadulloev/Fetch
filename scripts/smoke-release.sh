#!/usr/bin/env bash
set -euo pipefail

binary="${1:?usage: smoke-release.sh <fetch-binary>}"
smoke_root="$(mktemp -d)"
cleanup() {
  if [[ -n "${fetch_pid:-}" ]]; then kill "$fetch_pid" 2>/dev/null || true; fi
  rm -rf "$smoke_root"
}
trap cleanup EXIT
mkdir -p "$smoke_root/data" "$smoke_root/downloads"
config="$smoke_root/fetch.toml"
printf 'bind_address = "127.0.0.1"\nport = 18991\ndata_directory = "%s"\ndownload_directory = "%s"\nopen_browser_on_start = false\n' "$smoke_root/data" "$smoke_root/downloads" > "$config"
FETCH_CONFIG="$config" "$binary" >"$smoke_root/fetch.log" 2>&1 &
fetch_pid=$!
for _ in $(seq 1 100); do
  if curl --fail --silent http://127.0.0.1:18991/api/status | grep -q '"server":"ready"'; then
    exit 0
  fi
  if ! kill -0 "$fetch_pid" 2>/dev/null; then
    cat "$smoke_root/fetch.log" >&2
    exit 1
  fi
  sleep 0.1
done
cat "$smoke_root/fetch.log" >&2
exit 1
