#!/usr/bin/env bash
# Build the Rust binary and Python venv (idempotent).
set -euo pipefail
source "$(dirname "$0")/common.sh"

ROOT="$(resolve_root)"
echo "[concir-verify] repo root: $ROOT"

cargo build --release --bin cir2cvn --manifest-path "$ROOT/Cargo.toml"

if [ ! -x "$ROOT/python/.venv/bin/python" ]; then
  echo "[concir-verify] creating python venv..."
  python3 -m venv "$ROOT/python/.venv"
  "$ROOT/python/.venv/bin/python" -m pip install -q -r "$ROOT/python/requirements.txt"
fi

echo "[concir-verify] ready."
