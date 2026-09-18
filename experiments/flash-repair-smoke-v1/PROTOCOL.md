# Flash repair-type smoke — PROTOCOL (frozen 2026-09-18d)

Validates the repair-type multi-arm experiment where the defect is guaranteed by
construction. Inherits `experiments/EXPERIMENTS_V2_PROTOCOL.md`.

## Design change (deviation D-5)

The main experiment is **repair-type**, not generation-type: each task supplies a
defective program (Rust for A0/A1/A2, the corresponding CIR for A3) plus the
same natural-language requirement. Acceptance is each arm's own decision; the
terminal oracle is independent and reported in three separate columns.

## Tasks (3) and arms

- `lock-order/abba_2lock` — has a Rust reference; deadlock.
- `lock-order/partial_deadlock_bystander` — goal-layer defect.
- `condvar/bare_wait_no_predicate` — lost wakeup.

Arms: `A0_direct` (lower bound), `A1_self_iter`, `A2_tools_iter_m` (build+Miri),
`A2_tools_iter_ml` (build+Miri+Lockbud), `A3_ours_revision` (revise the buggy
CIR). Rust arms run only where a `rust/buggy.rs` exists (reported otherwise).

## Terminal oracle (three columns, never fused)

- `oracle.build` — `cargo build`;
- `oracle.miri` — Miri many-seeds (0..64) detection on the final artifact;
- `oracle.model` — for CIR artifacts, full Rust verification (`PASS` + preserved);
  for Rust artifacts, a model extraction is required and is reported `null` when
  not performed.
- `false_accept = accepted ∧ oracle_bug_present` and is `null` until a bug rule
  exists; "not detected" is never "correct".

## Budget

Model `deepseek-flash`, thinking disabled, temperature 0, `max_tokens=4096`,
timeout 90 s, SDK retries 0 plus <=1 transient. K=4. HTTP <= 48, wall <= 3600 s,
shared `budget.json`. Key from `.env` only.

## Known limits

- No `rust/tests/behavior.rs` is authored yet, so `oracle.behavior` is `null`.
- The Rust `bug_present` rule is per-task and not yet authored; false-accept for
  Rust arms is therefore reported `null` rather than inferred.
- A3's codegen/conformance path is exercised separately in
  `experiments/conformance-v1/` on correct CIR; channel cases are outside codegen.
