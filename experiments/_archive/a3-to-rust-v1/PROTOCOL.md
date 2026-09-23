# a3-to-rust-v1 — PROTOCOL (frozen 2026-09-2xk)

Give the A3 arms a Rust-side oracle, so all five arms align on the same Rust
columns. For every `accepted=True` A3_local / A3_whole cell of
`flash-repair-main-v1` (deduplicated by the accepted CIR sha256):

1. `concir-backend codegen` (BIN_MAIN `4bec943d…`) → skeleton + `labels.json`;
2. if the skeleton has holes, one LLM hole-fill (existing conformance prompt),
   one retry allowed (counted);
3. `cargo build` → `conform` (**strict codegen mode**: no `--lenient-unlock`,
   no `--attempt-events`) → behavior → Miri 16 seeds.

Behavior for a codegen skeleton is `terminated_ok` (exit 0) / `hang` (timeout) /
`crash` (non-zero); the CIR carries no `println`, so the task `DONE` terminal
line does not apply to the generated skeleton.

The result is written back to the main batch cell's `oracle.*` (new
`oracle.conform` = `PASS`/`FAIL`/`no_build`/`codegen`, with trace counts) and the
cell is queued for expert labelling. Repeated cells with the same CIR sha share
the result and are marked `dedup_of`.

Budget: <=110 requests; no `not_run` cells.
