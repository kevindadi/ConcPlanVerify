#!/usr/bin/env bash
# cir2cvn --analyze <file.json | ->
set -euo pipefail
source "$(dirname "$0")/common.sh"

ROOT="$(resolve_root)"
BIN="$(ensure_binary "$ROOT")"
INPUT="$(resolve_input "${1:--}")"
"$BIN" --analyze "$INPUT"
