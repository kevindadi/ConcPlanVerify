#!/usr/bin/env bash
# Python cir_workflow generate/repair/plan/merge.
# Usage: workflow.sh <command> [args...]
#   commands: generate | repair | plan | merge
set -euo pipefail
source "$(dirname "$0")/common.sh"

ROOT="$(resolve_root)"

if [ ! -x "$ROOT/python/.venv/bin/python" ]; then
  echo "[concir-verify] venv missing — run scripts/build.sh first" >&2
  exit 1
fi

cmd="${1:-}"
shift || true

if [ "$cmd" = "merge" ]; then
  # merge reads a bundle file/stdin; print merged JSON to stdout.
  PYTHONPATH="$ROOT/python" "$ROOT/python/.venv/bin/python" \
    -m cir_workflow merge "$@"
  exit $?
fi

if [ "$cmd" = "generate" ] || [ "$cmd" = "repair" ] || [ "$cmd" = "plan" ]; then
  PYTHONPATH="$ROOT/python" "$ROOT/python/.venv/bin/python" \
    -m cir_workflow "$cmd" "$@"
  exit $?
fi

echo "usage: workflow.sh <generate|repair|plan|merge> [args...]" >&2
exit 2
