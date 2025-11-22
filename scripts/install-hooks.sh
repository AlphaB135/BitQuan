#!/usr/bin/env bash
set -euo pipefail

# Configure repository to use .githooks as hooks path and install minimal pre-commit.

git config core.hooksPath .githooks
mkdir -p .githooks

if [ ! -f .githooks/pre-commit ]; then
  cat > .githooks/pre-commit <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
if command -v cargo >/dev/null 2>&1; then
  echo "[pre-commit] running cargo fmt --check"
  cargo fmt --all --check || { echo "Format check failed. Run: cargo fmt --all"; exit 1; }

  echo "[pre-commit] running cargo clippy"
  cargo clippy --all-targets --all-features -- -D warnings || { echo "Clippy failed"; exit 1; }

  echo "[pre-commit] running cargo test"
  cargo test --all --locked || { echo "Tests failed"; exit 1; }
fi
HOOK
  chmod +x .githooks/pre-commit
  echo "Installed .githooks/pre-commit"
else
  echo ".githooks/pre-commit already exists; leaving as is"
fi

echo "Git hooks installed (core.hooksPath=.githooks)."
