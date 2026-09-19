# Rust -> CIR extraction prompt (v4, labels only; Rust side is tool-generated)

The Rust has already been instrumented by the tool: a `cir_trace::ev` call was
inserted before every concurrency operation, and each call carries a label
`L<n>` from the table below. You do **not** write or edit Rust. Reply with
**exactly one fenced ```json block and nothing else** containing a ConcIR model.

Rules:

- Top level only `program` / `version` / `entry` / `modules` (no `contract`,
  `description`, or `notes`).
- Use only the schema's statement kinds and field names. Resource references and
  spawned/scope function references are `module::name` FQNs.
- Every concurrency statement's `sid` **must** be exactly one of the labels from
  the table (`L1`, `L2`, ...), placed on the matching operation. Non-concurrency
  statements (assignment, control flow, `return`) may use any unused `s<n>`.
- Thread tags from the table (`t0` main, `t1..` per spawn) tell you which
  function each event belongs to and in what order the threads are spawned.
- Model the program as written; do not "fix" it.

The annotated Rust already calls `cir_trace::finish()`; conformance is checked
with the implicit-unlock relaxation (a `mutex_unlock` step need not have an
event).
