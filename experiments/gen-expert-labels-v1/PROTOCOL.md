# gen-expert-labels-v1 — protocol

Agent-proxy rubric v3 over accepted generation Rust, deduplicated by ssh256.

- Input per cell: the requirement document (`REQUIREMENTS.md`) and the Rust
  program only. **No contract, no CIR** is shown to the annotator.
- Output per cell: `bug_present` (yes/no/unsure + line evidence) and, for every
  requirement `Ri`, `satisfied` (yes/no/unsure + line).
- Arms annotated in priority order (`G3_concir`, then `G0_direct`), capped at the
  Flash budget; cells beyond the cap are `not_run` in `labels.json`.
- The label is an author proxy, not a human verdict; the human queue is separate.
