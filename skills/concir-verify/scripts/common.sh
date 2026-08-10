#!/usr/bin/env bash
# Shared helpers for the concir-verify skill scripts.
set -euo pipefail

# Resolve the ConcPlanVerify repo root.
# Priority: $CONCPLANVERIFY_ROOT -> walk up from this script -> $PWD.
resolve_root() {
  if [ -n "${CONCPLANVERIFY_ROOT:-}" ]; then
    printf '%s\n' "$CONCPLANVERIFY_ROOT"
    return
  fi
  local dir
  dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  while [ "$dir" != "/" ]; do
    if [ -f "$dir/Cargo.toml" ] && [ -d "$dir/python/cir_workflow" ] && [ -d "$dir/src" ]; then
      printf '%s\n' "$dir"
      return
    fi
    dir="$(dirname "$dir")"
  done
  # Fall back to the current working directory (must be the repo).
  pwd
}

# Echo the release binary path, building it if missing.
ensure_binary() {
  local root="$1"
  local bin="$root/target/release/cir2cvn"
  if [ ! -x "$bin" ]; then
    echo "[concir-verify] building release binary..." >&2
    cargo build --release --bin cir2cvn --manifest-path "$root/Cargo.toml" >&2
  fi
  printf '%s\n' "$bin"
}

# Read the input argument ('-' = stdin) into a temp file, print the path.
resolve_input() {
  local arg="${1:--}"
  if [ "$arg" = "-" ]; then
    local tmp
    tmp="$(mktemp)"
    cat >"$tmp"
    printf '%s\n' "$tmp"
  else
    printf '%s\n' "$arg"
  fi
}
