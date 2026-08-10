#!/usr/bin/env bash
set -euo pipefail

cleanup() {
  jobs -p | xargs -r kill
}
trap cleanup EXIT INT TERM

cargo run --bin fetch &
(cd web && npm run dev) &
wait
