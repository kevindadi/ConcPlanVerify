#!/usr/bin/env bash
# Self-sufficiency check: the tracked evidence tier must regenerate RESULTS.md
# and tables/*.tex byte-for-byte from a clean `git archive HEAD`.
set -euo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

git -C "$REPO" archive HEAD | tar -x -C "$TMP"
cp "$REPO/experiments/RESULTS.md" "$TMP/ref_results.md"
mkdir -p "$TMP/ref_tables" && cp "$REPO"/experiments/tables/*.tex "$TMP/ref_tables/"

CMD=$(grep -m1 '^> python -m cir_workflow results' "$TMP/experiments/RESULTS.md" | sed 's/^> //')
# Normalise the duplicated subcommand token ("results results" -> "results").
CMD=$(printf '%s' "$CMD" | sed 's/cir_workflow results results/cir_workflow results/')
( cd "$TMP" && PYTHONPATH="$TMP/python" ${CMD} >/dev/null )

diff -q "$TMP/ref_results.md" "$TMP/experiments/RESULTS.md"
for f in "$TMP"/ref_tables/*.tex; do
  diff -q "$f" "$TMP/experiments/tables/$(basename "$f")"
done
echo "verify-evidence: OK"
