# extraction-v5 — PROTOCOL (frozen 2026-09-19j; finalized 2026-09-2xk)

Tool-driven extraction over every accepted A0/A1/A2 cell of
`flash-repair-main-v1`. `concir-instrument` annotates the Rust and emits
`labels.json`; the pinned Flash model writes **only** the CIR, mapping each
label to a statement `sid` (`prompts/rust_to_cir_extract_v5.md`). The model never
edits Rust.

## Validated definition (single source of truth: `CELLS.json`)

A cell is **validated** iff

```
stage == "explore"  AND  contract evaluation succeeded  AND
model_verdict in {PASS, FAIL}
```

`model_verdict == "INVALID"` (the contract could not be evaluated) is **not**
validated and is counted separately as `contract_invalid`. `SUMMARY.md` and
RESULTS are generated from `CELLS.json`; a mismatch between them is a bug.

Every `stage ∈ {codegen, labels, conform}` cell carries a non-empty `reason`
(first `E\d{3}` / panic line from the stored ConcIR stderr, or the harness
classification).

Conformance uses extraction mode (`--lenient-unlock --attempt-events`).
Budget: <=120 HTTP requests; `harness_error` must be 0.
