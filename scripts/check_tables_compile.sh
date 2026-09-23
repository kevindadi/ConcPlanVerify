#!/usr/bin/env bash
# Compile every experiments/tables/*.tex inside one minimal document; fail on
# any LaTeX error. Catches escaping bugs that a byte diff cannot.
set -euo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
TABLES_DIR="${1:-$REPO/experiments/tables}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

{
  echo '\documentclass{article}'
  echo '\usepackage{booktabs}'
  echo '\usepackage{longtable}'
  echo '\usepackage{array}'
  echo '\usepackage{amsmath}'
  echo '\begin{document}'
  for f in "$TABLES_DIR"/*.tex; do
    echo "\\input{$f}"
  done
  echo '\end{document}'
} > "$TMP/doc.tex"

( cd "$TMP" && pdflatex -interaction=nonstopmode -halt-on-error doc.tex >/dev/null )
echo "tables compile: OK"
