# extraction-v4 — PROTOCOL (frozen 2026-09-19j)

Tool-driven extraction: `concir-instrument` annotates the Rust candidate
(std-only, single file) with `cir_trace::ev` at every concurrency call and emits
`labels.json` (`L<n>` -> line/op/receiver/thread). The pinned Flash model is
asked for **only** the CIR, mapping each label to a statement `sid`
(`prompts/rust_to_cir_extract_v4.md`). The model never edits Rust.

Validation: build the annotated Rust against the generated `cir_trace` runtime,
trace it, and require every trace conformant under extraction-mode conformance
(`--lenient-unlock`: a guard drop may have no unlock event; `--attempt-events`:
lock/wait labels are recorded at the call site, i.e. the attempt step).

Budget: <=64 HTTP requests, wall <=3 h, thinking disabled, temperature 0,
max_tokens 4096, timeout 90 s. `harness_error` must be 0. Per cell
`extraction_result.json` records `stage` + JSON pointer + stderr path.

Prompt: `prompts/rust_to_cir_extract_v4.md` sha256 `da8702f07d7b84df09e407f09ea0060a54a49f981ad942da93ec4d61738a27fa`.
