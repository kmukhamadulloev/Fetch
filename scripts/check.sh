#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
if [ -f web/package.json ]; then
  (cd web && npm ci && npm run typecheck && npm run lint && npm run build)
fi
