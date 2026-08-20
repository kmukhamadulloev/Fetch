#!/usr/bin/env bash
set -euo pipefail
./scripts/test-release-notes.sh
if [ -f web/package.json ]; then
  (cd web && npm ci && npm run typecheck && npm run lint && npm test && npm run build)
fi
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace

if [ "${FETCH_RUN_E2E:-0}" = "1" ]; then
  (cd web && FETCH_E2E_PRODUCTION=1 npm run test:e2e)
fi
