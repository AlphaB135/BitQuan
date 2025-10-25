#!/usr/bin/env bash
set -euo pipefail

if command -v cargo >/dev/null 2>&1; then
  echo "[pre-commit] cargo fmt"
  cargo fmt --all -- --check

  echo "[pre-commit] cargo clippy"
  cargo clippy --all-targets --all-features -D warnings

  echo "[pre-commit] cargo test"
  cargo test --all --locked

  if [ -f Cargo.lock ]; then
    echo "[pre-commit] cargo deny"
    cargo deny check

    echo "[pre-commit] cargo audit"
    cargo audit
  else
    echo "[pre-commit] skipping cargo deny/audit (no Cargo.lock)"
  fi
else
  echo "cargo is not installed; skipping Rust checks" >&2
  exit 1
fi
