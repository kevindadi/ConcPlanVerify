# Rust -> CIR extraction prompt (v5, mandatory labels, CIR types only)

The Rust has already been instrumented by the tool: a `cir_trace::ev` call was
inserted before every concurrency operation, and each call carries a label
`L<n>` from the table. You do **not** write or edit Rust. Reply with **exactly
one fenced ```json block and nothing else** containing a ConcIR model.

## Mandatory labels

`labels.json` is a **checklist**: every label in the table must appear **exactly
once** as a statement `sid` in your CIR, on the matching operation. If any label
is missing or duplicated the reply is rejected. Non-concurrency statements
(assignment, control flow, `return`) use `s<n>`.

## CIR types only (no Rust types)

Allowed resource `type` values: `Mutex`, `Condvar`, `Semaphore`, `Channel`,
`Atomic`, `Var`. Allowed `base`/local/param types: `Int`, `Bool`, `Float`,
`String`.

- `"type": "Mutex"` ✓   `"type": "Arc<Mutex<i32>>"` ✗
- `"base": "Int"` ✓     `"base": "Vec<i32>"` ✗
- values are CIR expressions (`"count + 1"`, `"true"`), never Rust code
  (`Arc::new(...)`, `vec![...]` are rejected).

## Structure

- Top level only `program` / `version` / `entry` / `modules`; `version` is
  exactly `"3.5.0"`.
- `modules`, each module's `resources` / `functions` / `protection`, and each
  function's `body` are JSON arrays, never objects keyed by name.
- Resource references and spawned/scope function references are `module::name`.
- `branch.then`, `branch.else`, `switch.default` are single `sid` strings.
- Declare every local you assign to in the function's `locals`; `assign_local`
  targets and `read_shared.dst` must be declared locals or parameters.
- Never assign to a sync resource or atomic.

Model the program as written; do not "fix" it. The annotated Rust already calls
`cir_trace::finish()`; conformance uses the implicit-unlock and attempt-event
relaxations.
