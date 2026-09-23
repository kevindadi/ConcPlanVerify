# EXPERIMENTS V2 — offline handoff (2026-09-18)

Status: **offline harness implemented and exercised. No live model request has
been made.** The live DeepSeek Flash batch is gated on the user confirming
[`EXPERIMENTS_V2_PROTOCOL.md`](EXPERIMENTS_V2_PROTOCOL.md).

## What is frozen

- Protocol: `experiments/EXPERIMENTS_V2_PROTOCOL.md` (arms, K=4, miri table,
  budgets, oracle rules, threats). Any change must be reported as a deviation.
- Benchmark manifest: `benchmarks/MANIFEST.json` (sha256 per file, per-task
  `status`).
- Harness constants: `python/cir_workflow/experiments_v2.py` (`K`, miri combos,
  live caps, arm ids).

## What is implemented (offline)

| module | purpose |
| --- | --- |
| `python/cir_workflow/experiments_v2.py` | consumption accounting, arm/round records, manifest loader with hash verification, derived false-accept / behavior-loss / conservative-reject |
| `python/cir_workflow/rust_arm.py` | isolated std-only cargo project; `cargo build`/`test`/`miri`/`lockbud` subprocesses, timeouts, offline, raw output hashed |
| `python/cir_workflow/revision_workflow.py` | arm A3 whole-artifact CIR revision; version archive + hashes; ablation switches (`nodiag`, `nopreserved`, `nofidelity`) |
| `python/cir_workflow/arms.py` | arm glue (A0/A1/A2 Rust; A3 family), terminal oracles, offline batch, fail-closed live gate |
| `python/cir_workflow/detection.py` | Track D (ConcIR explore + miri + lockbud) |
| `python/cir_workflow/scale.py` | B5 generated lock-chain scale/cost |
| `benchmarks/build_patterns.py` | reproducible benchmark authoring + manifest hashing |

Tests: full suite **72 passed** (`PYTHONPATH=python:. python3 -m unittest
discover -s python/tests -t .`), including the new `test_experiments_v2`,
`test_rust_arm`, `test_revision_workflow`, `test_arms`. Rust: **229 passed**.

### Offline exercises (real, no network)

- `experiments/flash-arms-v1/offline/ARMS_OFFLINE.json` — scripted P1/P4
  whole-CIR revision reaches `accepted` at round 1; scripted Rust A0 builds.
- `experiments/detection-v1/DETECTION.{md,json}` — Track D.
- `experiments/scale-v1/SCALE.{md,json}` — B5.

## Track D result (headline)

| task | ConcIR buggy | ConcIR fixed | Miri buggy (5 seeds) |
| --- | --- | --- | --- |
| P1 | FAIL | PASS | not detected |
| P4 | FAIL | PASS | not detected |
| rmw-zenoh-998 | FAIL | PASS | n/a (CIR only) |
| dashmap-369 | UNSUPPORTED | n/a | n/a |

ConcIR detects both lock-order bugs and their fixes; Miri did not flag either
lock-order bug in any of the 5 frozen schedules. This is expected for
schedule-dependent lock-order cycles and is **not** evidence of safety. Lockbud
is not installed (`cargo lockbud` absent), so that arm is recorded
`unavailable`; no substitute tool was used.

## B5 result (headline)

Generated global-order lock chains: `threads x chain` states grow from 39
(2x2) to 78\,263 (6x3, 2.6 s). The UNKNOWN boundary is observable, e.g. 5x2 at
`max_states=5000` is `UNKNOWN`, 5x3 at `max_states=20000` is `PASS`, and 6x3 at
`max_states=20000` is `UNKNOWN`. UNKNOWN is a bound, never a verdict.

## Benchmark status

| task | status | notes |
| --- | --- | --- |
| P1 two-mutex deadlock | **ready** | CIR validated FAIL/PASS; contract; rust |
| P4 three-lock circular | **ready** | CIR authored + validated FAIL/PASS |
| P2 signal loss | to_author | rust + legacy brief present; modular CIR/contract pending |
| P3 channel + mutex | to_author | same |
| P5 partial deadlock | to_author | same (goal-layer defect; needs reachability contract) |
| P6 dual condvar | to_author | same |
| P7/P8/P9 baselines | to_author | rust present; CIR/contract pending |
| rmw-zenoh-998 | **ready** | CIR + contract imported |
| dashmap-369 | **ready** | expected UNSUPPORTED (RwLock) |

`to_author` tasks are excluded from the live batch and reported as such, not
faked. Authoring them is the main prerequisite for a full P1–P9 live batch.

## Not yet done (blockers for a full live batch)

1. Author the remaining modular CIR + contracts for P2/P3/P5/P6/P7/P8/P9 and the
   per-task behavioral `cargo test`s (expected outcomes). The oracle currently
   returns `behavior_test_ok=None` and `bug_present=None` for Rust artifacts
   because those tests/rules are not authored; it never guesses.
2. Rust generation/review prompt assets for A0/A1/A2 live (offline uses scripted
   text). The CIR arm already has the live provider path via `live.py`.
3. Live entry point wiring for `arms.py` (currently fail-closed via
   `assert_protocol_confirmed`).

## Reproduction

```bash
# offline tests
PYTHONPATH=python:. python3 -m unittest discover -s python/tests -t .

# Track D (no LLM)
PYTHONPATH=python python3 -m cir_workflow --out experiments/detection-v1 detection

# B5 (no LLM)
PYTHONPATH=python:. python3 -m cir_workflow --out experiments/scale-v1 scale

# rebuild the benchmark manifest (after authoring new task files)
python3 benchmarks/build_patterns.py
```

## Environment drift (external, not part of this work)

During this round an external uncommitted edit changed `ConcIR/Cargo.toml`
`edition = "2021"` -> `"2024"` and deleted `ConcIR/doc/todo.md`. Under edition
2024 the current sources fail to compile (`cannot explicitly borrow within an
implicitly-borrowing pattern` in `ast.rs`, `validate/control.rs`,
`validate/types.rs`), so a fresh `cargo test` no longer builds. This is
unrelated to the migration: the A-phase acceptance run right after the move
recorded **229 passed / 0 failed / 0 errors** (saved at
`/var/folders/.../rust-tests-a.txt`). Leave the `Cargo.toml` decision to the
owner; if edition 2024 stays, those three patterns need fixing first.

## Threats to validity

See the protocol. In short: single model (Flash), small sample, Miri is dynamic
and non-exhaustive, the ConcIR detection arm consumes CIR rather than source, and
the Rust terminal oracle is incomplete until the behavior tests are authored.

---

# Round 2026-09-18c — review fixes, capability benchmark, smoke batch

## REVIEW.md regression results

| review item | status | evidence |
| --- | --- | --- |
| P1-1 raw tool output not archived | fixed | every tool call writes `calls/<seq>-<tool>/{argv,env,stdout,stderr,exit,wall_ms}`; records carry paths + hashes. See `experiments/detection-v2/rust_projects/*/calls/*/env.json` (contains `MIRIFLAGS`). |
| P1-2 zero-test project counted as pass | fixed | `parse_test_result` requires `test result: ok. N passed` with N≥1; zero tests -> `behavior_test_ok=null`, reason `no_tests`. Regression in `test_rust_arm.py`. |
| P1-3 ConcIR not compilable / stray files | resolved externally | `rust-toolchain.toml` correctly named (working tree channel `nightly`), edition 2024 retained, `cargo test --offline --all-targets --no-fail-fast` = 229 passed / 0 failed. |
| P2-1 benchmark covered 2/9 | replaced | capability-family benchmark with **21 ready cases** across 8 families; P1-P9 retired to `benchmarks/legacy-paper-patterns` (`status: legacy`). |
| P2-2 P1 provenance | addressed | `ground_truth.json` declares the buggy CIR as `llm_generated` pilot-v1 t2_abba (`d2d4958b...`). |
| P2-3 miri sample size / error class | fixed | one `-Zmiri-many-seeds=0..64` pass added (D-1); exit≠0 without a deadlock/race match is `tool_error`, not `detected`; Miri error records are separate. |
| P2-4 A3 rounds empty | fixed | revision workflow records per-round request/feedback/tool hashes, tokens, LLM/tool wall; `run_cir_arm` maps them into `ArmRun.rounds`. Regression: three-round FAIL→static-error→PASS has 3 records. |
| P2-5 env not visible | fixed | `env.json` per tool call records `MIRIFLAGS`, `CARGO_*`, `RUST_BACKTRACE`. |

## Versions

- ConcIR binary: `fe3d22c5a52f84b5a46d7a4665328056cec9b132f33eec9ea52c3c1158b7d12d`
  (`target/release/concir-backend`, built from the committed tree before the
  last unstaged `thiserror`/comment cleanup; semantics unchanged).
- `rustc 1.100.0-nightly (a69a63265 2026-09-03)`, edition 2024.
- `miri 0.1.0 (a69a63265c 2026-09-03)`; `-Zmiri-many-seeds` supported.
- lockbud: built from `tools/lockbud` (commit `cc78cb72cb85cb80e596717339cca95ef50c8fe0`,
  `nightly-2026-02-07`), wrapper sha256 `a6cc62c0ea6...`; run as `RUSTC_WRAPPER`
  with `-k deadlock -l cir_arm_probe`, output archived under `calls/*-lockbud`.

## Capability-family benchmark

`benchmarks/MANIFEST.json`: 30 tasks (21 ready, 9 legacy). Families:
lock-order, condvar, channel, semaphore, atomic-data, structure, boundary,
real-cases. `benchmarks/FAMILIES.md` lists the matrix and the capabilities not
covered by the paper's Table 1 (precise condvar wait-set, bounded channel,
counting semaphore, bounded-Int safety invariants, `scope`/`bound`, `AG EF`,
UNSUPPORTED/UNKNOWN boundary controls). Every ready case passed
`check`/`support`/`explore` with **petri == interp** and **0 pre-registration
mismatches** (`python3 benchmarks/build_families.py`).

Partial-deadlock preregistration confirmed: `deadlock_free = PASS` while
`always_reachable(a/b) = FAIL` on the buggy case, `PASS` on the fixed case.

## Track D v2

`experiments/detection-v2/` (new binary, raw evidence archived, Miri N=65 per
program: 5 frozen seeds + one 64-seed pass). Across the 21 ready cases: 11
buggy/reference CIR cases FAIL, 7 fixed/reference cases PASS, 1 correct seed
PASS, 3 UNSUPPORTED (RwLock, async/await, dashmap), 1 UNKNOWN (unbounded Int),
and 9 legacy patterns are skipped. Miri did not flag either lock-order bug across
65 seeds (dynamic, schedule-dependent; not safety). Lockbud (now built, see
below) flagged `ConflictLock` on `lock-order/abba_2lock` (buggy) and its fixed
twin clean, but on `lock-order/cycle_3lock` it reported a `DoubleLock` on the
**fixed** program (false positive) and nothing on the buggy one (false negative)
— recorded as-is. Lockbud is a static, "possibly" over-approximation; the two
lock-order fixtures show it is neither sound nor complete here.

## Lockbud integration

`tools/lockbud` (separate checkout, commit `cc78cb7`) is built in place with its
own `rust-toolchain.toml` (`nightly-2026-02-07`); the harness locates
`tools/lockbud/target/release/lockbud` (override with `LOCKBUD_BIN`), compiles
each candidate in a dedicated `lockbud-probe/` with `RUSTC_WRAPPER` set and
`LOCKBUD_FLAGS="-k deadlock -l cir_arm_probe"`, and classifies by parsed
`bug_kind` records only (Lockbud always prints a zero-count `conflictlock`
summary, so a text scan would false-positive every run). `tools/` is ignored by
`.gitignore` (vendored third-party; pin by commit). The repository `.gitignore`
also ignores `*.txt`, so the archived `stdout.txt`/`stderr.txt`/`exit.txt`
evidence is intentionally **not git-tracked** (repository policy); the files
stay on disk and their sha256 is recorded in the JSON records
(`stdout_sha256`/`stderr_sha256` plus `files` paths), so they remain
integrity-checkable without being committed. Note: Lockbud's
`DoubleLock` detector fires on any thread that holds one lock while taking
another, so it false-positives on the correct 3-lock and partial-deadlock
programs; A2 treats a Lockbud detection as "not green" and keeps iterating.

## Flash smoke batch

`experiments/flash-arms-smoke-v1/run-20260918T123238-73821-fdfe0d/` is the
delivered batch (protocol sha `2429...c8ff`, **36 / 48 requests**, stop reason
none). Arms: A0/A1/A2/A3/A3p/A3_tool_repair on 3 tasks; A2 now includes Lockbud.

- `lock-order/abba_2lock`: A0 accepted r1; A2 accepted r1 (build + Miri 5 seeds +
  64-seed pass + Lockbud all clean); A3 accepted r1 (whole-CIR revision); A3p
  accepted + replayed; A3_tool_repair `repaired`.
- `condvar/lost_wakeup_notify_before_wait`: A0 accepted r1 and A2 accepted r1
  **because the model wrote a correct program on the first try** (predicate loop,
  flag set before `notify`); this is not a Miri/Lockbud miss and the earlier
  "both tools miss the lost wakeup" reading was wrong (REVIEW C-3). A3 stopped
  at round 1 with `check_status=usage_error` because the model wrote
  `write_shared.expr` as an object; the harness treated it as a tool error
  instead of schema feedback (REVIEW C-2), and the earlier "revision did not
  reach a complete PASS" wording was also wrong. Both are fixed this round.
- `lock-order/partial_deadlock_bystander`: A0 accepted r1; A1 and A2 not
  accepted (Lockbud reports `DoubleLock` on the generated program every round —
  an over-approximation, so A2 correctly refuses to call the tools "green");
  **A3 accepted at round 4** (the whole-CIR revision reached a complete PASS on
  the fourth proposal); A3p `rejected`; tool_repair
  `no_acceptable_candidate`.

Superseded batches retained in the same directory: `run-20260918T115401…` (A3
wall-clock not aggregated), `run-20260918T120310…` and `run-20260918T122307…`
(A2 green logic / Lockbud status read at the wrong level). The final batch uses
36 requests; each superseded batch used ≤42.

## ConcIR status and commit suggestions

- Working tree (external edits, not mine): `Cargo.toml` removes unused
  `thiserror`, `rust-toolchain.toml` channel `nightly-2025-10-27` -> `nightly`,
  `src/ast.rs` removes a section comment. All 229 tests pass.
- Suggested split: (1) already committed `816b637` edition 2024 + experiment
  removal and `6105d40` toolchain file; (2) a small `chore: drop unused thiserror
  and stray comment` for the remaining unstaged edits. No code semantics changed.
- `doc/backend-usage.md` now records the pinned toolchain and the 229-test run.

## Repro

```bash
python3 benchmarks/build_families.py
CONCIR_BACKEND=/Users/kevin/local-repos/ConcIR/target/release/concir-backend \
  PYTHONPATH=python python3 -m cir_workflow --out experiments/detection-v2 detection
PYTHONPATH=python python/.venv/bin/python -m cir_workflow \
  --binary /Users/kevin/local-repos/ConcIR/target/release/concir-backend \
  --out experiments/flash-arms-smoke-v1 smoke \
  --protocol experiments/flash-arms-smoke-v1/PROTOCOL.md \
  --protocol-sha256 242984312502e5b2586021ffe974232028c3caada8737482738264bc0425c8ff
```

## Remaining gaps

- Rust `bug_present` in the terminal oracle is still `null` where no per-case
  rule exists; behavior tests are authored only where noted. The oracle never
  infers safety from a detector miss.
- Two of the requested real-case reductions and several family cases
  (`cycle_3lock` variants, `rendezvous`, `nested_scope`, extra condvar/channel
  cases) are not all present; the manifest records exactly what is ready.
- `A3_nofidelity`, full multi-arm live, and multi-model comparison remain out of
  scope for this round.

---

# Round 2026-09-18d — review fixes, conformance layer, repair benchmark

## REVIEW C-1..C-5 regression results

| item | status | evidence |
| --- | --- | --- |
| C-1 A1 never accepts `NO_ISSUES` | fixed | `run_rust_arm` parses the reply: exactly `NO_ISSUES` -> `decision=self_no_issues`, accepts the current candidate without compiling it. Regression `test_a1_no_issues_accepts_without_compiling_sentinel`. |
| C-2 A3 schema error stops the batch | fixed | a `check` usage/protocol error becomes `stage=check, schema_error` feedback and spends the round; only a missing/broken binary is a tool error. Regression `test_schema_error_is_feedback_not_tool_error`. Prompt now states `expr` is a string with an example. |
| C-3 HANDOFF misread lost_wakeup | fixed | the "Miri/Lockbud miss the lost wakeup" and "A3 did not reach PASS" sentences are corrected: the model wrote a correct program, and A3's first candidate was schema-invalid. |
| C-4 SUMMARY arm/id mismatch | fixed | A3p writes to `A3p_ours_patch/`, the renderer prints A3p/tool-repair status from their own payloads. |
| C-5 fragments accepted | fixed | a non-sentinel reply without `fn main` is `format_error` with format feedback. Regression `test_a1_fragment_is_format_error`. |
| P2 same_cv label | fixed | `condvar/same_cv_different_locks` now has only `correct.cir.json`; stale `buggy.cir.json` removed on rebuild. |
| P2 `.gitignore` | fixed | `!experiments/**/*.txt` re-includes archived tool evidence. |
| P2 A2 columns | fixed | per-round `build_ok`/`test_ok`/`miri_green`/`lockbud_green`; A2-m and A2-ml arms. |
| P2 toolchain | partially | `rust-toolchain.toml` currently `channel = "nightly"`; the resolved toolchain is `rustc 1.100.0-nightly (a69a63265 2026-09-03)` / `miri 0.1.0 (a69a63265c)`. `nightly-2026-09-03` is not installed under that name, so it was not pinned (deviation). |

## Conformance layer (D-6)

ConcIR adds `codegen` + `conform` and an emitted `cir_trace` runtime; ConcPlanVerify
adds `python/cir_workflow/conformance.py`. The rule: a trace event is a
**completed** step for lock/acquire/condvar-wait/channel, and a reached
statement for unlock/notify/release/scope/spawn/join. `conform` carries a
frontier of candidate model states and assigns child tags during silent
spawn/scope steps, so nondeterministic bindings and blocking attempts are handled.

`experiments/conformance-v1/CONFORMANCE.md` (offline, no LLM; 50 native runs + one
Miri many-seeds pass per case):

| case | traces | conformant | violation | coverage |
| --- | --- | --- | --- | --- |
| lock-order/abba_2lock (fixed) | 51 | 51 | 0 | 4/4 |
| condvar/lost_wakeup (fixed) | 51 | 51 | 0 | 4/4 |
| semaphore/permit_leak (fixed) | 51 | 51 | 0 | 2/2 |
| structure/scope_bound_k_workers (correct) | 51 | 51 | 0 | 2/2 |

A tampered trace (a lock-out-of-order or an unknown sid) is reported
`violation` / `unknown_sid` with the event index; Rust unit tests in
`tests/conformance.rs` cover this. The Rust suite is **234 passed / 0 failed**
(229 + 5 conformance tests); Python is **88 passed**.

## Capability benchmark

`benchmarks/MANIFEST.json` now has **37 ready cases + 9 legacy**. Every family has
at least two buggy cases: lock-order 5, condvar 3, channel 3, semaphore 2,
atomic-data 2, structure 2, boundary 2, real-cases 2. New cases:
`semaphore/acquire_twice_no_release`, `condvar/bare_wait_no_predicate`,
`channel/bounded_backpressure_lock_held`, `atomic-data/counter_overflow_safety`,
`atomic-data/atomic_lost_update`, `structure/nested_scope_lock_order`,
`structure/scope_worker_abba`. Each buggy case also writes a `repair_task.json`.
All cases pass `check`/`support`/`explore` with **petri == interp** and **0
pre-registration mismatches** (`python3 benchmarks/build_families.py`).

## Repair-type smoke — completed

Delivered batch `experiments/flash-repair-smoke-v1/run-20260918T161254-8755-bf0e2a/`
(protocol sha `0811f44c…`, **30 / 48 requests**, stop reason none; K=4). Miri is
bounded (8 s per run, no many-seeds) so a still-deadlocking artifact is recorded
as `timeout`, not a hang.

| task | A0 | A1 | A2-m | A2-ml | A3 (round) | A3 oracle |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | accept r1 | accept r2 | accept r1 | accept r1 | **accept r2** | verify PASS, bug_present False |
| lock-order/partial_deadlock_bystander | accept r1 | reject | reject | reject | reject | verify FAIL, bug_present True |
| condvar/bare_wait_no_predicate | accept r1 | accept r2 | accept r1 | accept r1 | reject | verify FAIL, bug_present True |

- **Measurable A3 acceptance**: on `abba_2lock` the whole-CIR revision reached a
  complete `PASS` at round 2 and its `oracle.model` is `verify_pass=True,
  bug_present=False`; the other two A3 runs are correctly rejected with the final
  CIR still failing (`bug_present=True`).
- Rust arms: `oracle.build` and `oracle.miri` are populated; `miri` did not
  detect any of these defects (expected: lost wakeup / goal-layer / schedule
  dependent). `A2_tools_iter_ml` correctly refused `partial_deadlock_bystander`
  because Lockbud reports `DoubleLock` on the generated program.
- `false_accept` is computed where a `bug_present` rule exists (A3); for Rust
  arms it stays `null` because the per-task `bug_present` rule is not authored.

Two earlier attempts (`run-20260918T140756…`, `run-20260918T150907…`) were
aborted by unbounded Miri on hanging programs; the harness fix (single-seed Miri
now honours the analyzer timeout) is what made this run complete. Those batches
are retained.

## Versions / binary

- ConcIR release binary sha256 `eedf7bfe0d5e6faf1293c66e08a595cdea4ace457f6390fcd38066f6265fad94`.
- `rustc 1.100.0-nightly (a69a63265 2026-09-03)`, `miri 0.1.0 (a69a63265c 2026-09-03)`.
- Lockbud commit `cc78cb72cb85cb80e596717339cca95ef50c8fe0`, toolchain `nightly-2026-02-07`.

## Repro

```bash
python3 benchmarks/build_families.py
PYTHONPATH=python python3 -m cir_workflow --binary <concir-backend> \
  --out experiments/conformance-v1 conformance
# repair smoke (Miri bounded to 8 s per run, no many-seeds)
PYTHONPATH=python python/.venv/bin/python -m cir_workflow --binary <concir-backend> \
  --out experiments/flash-repair-smoke-v1 repair-smoke \
  --protocol experiments/flash-repair-smoke-v1/PROTOCOL.md --protocol-sha256 <sha>
```

## Remaining gaps

- Repair smoke `SUMMARY` is complete; Rust arm `bug_present`/behavior oracles
  remain `null` (rules not authored), so Rust-arm false-accept is not computable.
- Rust `bug_present` rules and `rust/tests/behavior.rs` are still not authored,
  so `oracle.model`/`oracle.behavior` are `null`; false-accept for Rust arms is
  not yet computable.
- `codegen` supports a single module and no channel/RwLock/composite values; the
  channel and nested-scope buggy cases are Track D only.
- The model-extraction oracle (LLM extracts Rust back to CIR) is not implemented.
- `rust-toolchain.toml` is not pinned to an immutable date (deviation).

---

# Round 2026-09-18e — de-leak, normalisation, conformance口径

## REVIEW R-1..R-8 status

| item | status | evidence |
| --- | --- | --- |
| R-1 input leakage | fixed | `benchmarks/build_families.py` emits `repair_input/` (comments stripped + rustfmt, `program` anonymised to `case_<family><n>`, neutral `requirements.txt`); `python/tests/test_sanitize.py` lints it. |
| R-2 A3 loses on schema | fixed | ConcIR `schema` + `prompts/concir_generation_v2.md`; `normalize.py` Tier-1 rewrites with records; schema feedback uses JSON pointers + shapes; stall -> local patch -> `stalled`. Offline regression: smoke-d `bare_wait` v3/v4 raw text now normalises to **check valid + explore PASS** (`tests/test_normalize.py::SmokeDRegressionTests`). |
| R-3 diagnostic readability | **not done** | `related_functions`/`blocked` still numeric and there is no `doom_state`; target-layer feedback is unchanged. See remaining items. |
| R-4 coverage/traces | fixed | coverage per `(function, sid)`; one trace per Miri seed (`cargo clean` before each so `CIR_TRACE_OUT` applies), `traces_total = native + seeds`, timeouts named `timeout` with `hang_suspect`. |
| R-5 Miri `tool_error` | partially | Miri is bounded (8 s, no many-seeds) in the arm oracle; the A2-ml `tool_error x5` root cause was not investigated (Lockbud wrapper env suspicion unconfirmed). |
| R-6 codegen coverage | partially | channel codegen added (emitted `cir_trace::Channel`); `real-cases/rmw-zenoh-998` codegens and builds; **multi-module codegen not implemented**. |
| R-7 HOLE fill / A3_free | partially | `worker_with_payload` (nobody helper) now yields a real codegen HOLE and `structure/worker_with_payload/correct.cir.json` is `conformance_only`. The live Flash fill and the `A3_free` ablation were **not run**. |
| R-8 repo hygiene | fixed | ConcIR sources committed (`f7fa67f`), toolchain pinned (`rust-toolchain.toml = nightly-2026-09-04`, miri+rust-src, 234+ tests); ConcPlanVerify committed in groups. |

## Conformance v2 (new口径)

`experiments/conformance-v2/CONFORMANCE.md` — 50 native + 12 Miri-seed traces per
case, coverage keyed per `(function, sid)`:

| case | traces | conformant | violation | timeout | coverage |
| --- | --- | --- | --- | --- | --- |
| abba_2lock (fixed) | 62 | 62 | 0 | 0 | 9/9 |
| lost_wakeup (fixed) | 62 | 62 | 0 | 0 | 7/7 |
| permit_leak (fixed) | 62 | 62 | 0 | 0 | 5/5 |
| scope_bound (correct) | 62 | 62 | 0 | 0 | 7/7 |

Channel codegen produces buildable programs; channel **conformance** is not yet
claimed — the reference model attributes a rendezvous to a different step than
the emitted `ev`, so the two sides do not line up (documented gap).

## Extra finding

`worker_with_payload` exposed a **petri vs interp disagreement** on a `call` to a
body-less helper (`petri FAIL`, `interp PASS`). The case is therefore
`conformance_only`, not an explore benchmark; the disagreement is recorded for a
future ConcIR fix.

## Not done this round

- **R-3** FQN rendering + `doom_state` + templated repair hints (ConcIR).
- **R-6** multi-module codegen.
- **R-7** live HOLE fill on `worker_with_payload` and the `A3_free` ablation.
- **VI.1/2/3** injectable `rust/tests/behavior.rs` for six cases, `bug_present`
  rules, and the LLM extraction oracle; consequently the repair-smoke Rust arms
  still have no `oracle.model`/`oracle.behavior` values and no false-accept rate.
- **VII** repair-smoke-v2 was not rerun.
- R-5 root cause of `tool_error x5`.

## Versions

- ConcIR release binary sha256 `5aac4ac1851f40b25e58ae4337f82a6e6c11ac41831b2b9362161de88cc60865`.
- `rustc 1.100.0-nightly (a69a63265 2026-09-03)` under pinned `nightly-2026-09-04`.
- Rust tests **237 passed**, Python **99 passed**.

---

# Round 2026-09-18f — first credible repair data

## N-1..N-4 and remaining items

| item | status | evidence / commit |
| --- | --- | --- |
| N-1 petri/interp on empty-body call | fixed | implicit return in both engines; `E114 FallOffEnd` warning; `tests/engine_agreement.rs`; `worker_with_payload` PASS in both engines. ConcIR `722f271`, `43679b9`. |
| N-2 Miri `tool_error` root cause | fixed | it was a thread leak; `thread_leak` classification + regression. ConcPlanVerify `9081686`. |
| N-3 channel conformance | recorded | codegen emits completion events; the runtime rendezvous order (receiver-first) diverges from the model's second-arriver attribution; channel cases are violations, not excused. ConcIR `f9df7fa`. |
| N-4 version binding | fixed | `binary_sha256` + `git_rev` in `explore`/`codegen`/`conform` output; build.rs watches the branch ref; conformance JSON carries `binary_sha256`. |
| R-3 diagnostics | fixed | `counterexample_names`, `doom_state` (holds/waiting_on FQN, free_resources), templated loop hints. ConcIR `1f7cb07`. |
| R-6 multi-module codegen | fixed | `mod <name> { pub(crate) fn ... }` + `crate::`-qualified refs; authored a `fixed` twin for `cross_module_cycle` (unified cross-module order). ConcIR `79c9440`. |
| VI.1 behavior.rs x6 | substituted | behaviour column is a bounded run of the built candidate (deviation D). |
| VI.2 arm Miri many-seeds | fixed | per-seed 0..15 in the arm oracle. |
| VI.3 extraction oracle | implemented, weak | prompt + `extract.py`; most extractions fail codegen validation and are `extract_unverified` (honest). |

## Engine agreement

`CONCIR_ENGINE_AGREEMENT_DIRS=benchmarks/families:benchmarks/real-cases cargo
test -p concir --test engine_agreement` → **zero disagreements** over 38 family
cases, real-cases, examples and tests fixtures.

## Diagnostics regression

`partial_deadlock_bystander` buggy diagnostic now carries the symmetric doom
state: `main::a at s4 holds [main::a] waiting_on mutex main::b` and
`main::b at s4 holds [main::b] waiting_on mutex main::a`, plus two templated
cycle hints. `counterexample_names` are `module::function::sid`.

## conformance-v3

`experiments/conformance-v3/CONFORMANCE.{json,md}` (binary
`6ff6dad8…`, git_rev `1f7cb07…`):

| case | traces | conformant | violation | timeout | coverage |
| --- | --- | --- | --- | --- | --- |
| abba_2lock (fixed) | 58 | 58 | 0 | 0 | 9/9 |
| lost_wakeup (fixed) | 58 | 58 | 0 | 0 | 7/7 |
| permit_leak (fixed) | 58 | 58 | 0 | 0 | 5/5 |
| scope_bound (correct) | 58 | 58 | 0 | 0 | 7/7 |
| rendezvous_both_send (fixed) | 58 | 0 | 58 | 0 | 0/0 |
| bounded_backpressure (fixed) | 58 | 49 | 9 | 0 | 5/5 |
| rmw-zenoh-998 (buggy) | 58 | 24 | 0 | 34 | 9/9 |
| worker_with_payload skeleton_fill | 66 | 66 | 0 | 0 | 5/5 |
| worker_with_payload A3_free | 66 | 0 | 0 | 0 | 0/0 (all missing traces — the free program never called `cir_trace::finish`) |

The four v2 cases stay at 100% with the new `(function, sid)` coverage. The
channel violations are a codegen-vs-model rendezvous ordering gap (analysis in
`docs/backend-usage.md`); `rmw` timeouts are the buggy program hanging, which is
the expected observation.

## repair-smoke-v2 (de-leaked inputs)

Delivered batch `experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/`
(protocol sha `b7da4a11…`, **40 / 72 requests**, stop reason none). Three oracle
columns per Rust arm; A3 uses the CIR verdict.

Highlights (full table in `SUMMARY.md`, incl. the v1→v2 comparison):

- **A measurable false-accept**: `A0_direct` on `partial_deadlock_bystander`
  accepted round 1, `oracle.behavior = hang`, `bug_present = true` →
  `false_accept = true`. After de-leaking, the direct arm no longer sees the
  answer and its fix still hangs.
- **A3 now accepts all three**: abba round 2, partial round 3, bare_wait round 4
  (v1 rejected partial and bare_wait). Decisions: abba `explore_fail → accepted`;
  partial `explore_fail ×2 → accepted`; bare_wait `explore_fail → check_invalid ×2
  → accepted` (the schema normaliser rescued the `check_invalid` rounds).
- A2-m/A2-ml on partial need 4 rounds after de-leaking (v1 needed 1), and all
  end `terminated` (recorded as `terminated, unverified`; the old candidates
  predate the observable-terminal requirement, so they print no terminal line).
- `oracle.model` is `extract_unverified` for most Rust arms (the extracted CIR did
  not codegen); `oracle.miri` detected nothing. `inconclusive` cells are those
  where build succeeded but extraction failed — recorded as is.

Exact tallies over the 12 Rust-arm cells (3 tasks × 4 arms): accepted **11/12**,
behaviour `terminated` **10**, `hang` **1**, `false_accept` **1**,
`oracle.model` validated **0/12 → inconclusive ratio 100%** (the extraction CIRs
failed codegen, which is the concrete reason to add multi-module codegen), and
`oracle.miri` detected **0**.

## Commits this round

- ConcIR: `722f271` (N-1 + E114 + engine_agreement + version fields),
  `43679b9` (build.rs ref watch), `1f7cb07` (doom-state diagnostics),
  `f9df7fa` (channel doc/rule).
- ConcPlanVerify: `9081686` (oracle: thread_leak, per-seed Miri, behaviour,
  extraction), `45b27a0` (repair-smoke-v2 + conformance-v3).

## Versions

- Conformance-v3 and repair-smoke-v2 were produced by binary
  `6ff6dad887dc6ea1eb4cd446be400f8ae26411a6abbda04215c0f0b9cd4d8637` at
  git_rev `1f7cb07acfdd001c2d65b9532bebdb0134b99099`; the conformance-v3 JSON
  records exactly that pair (HANDOFF and JSON agree). The later commit
  `f9df7fa` is docs-only and, when rebuilt, gives a different binary hash
  (`b12c813e…`) with git_rev `f9df7fa…`.
- Rust tests **238 passed**; Python **100 passed**.

## Section 6 (optional) — multi-module codegen

`cross_module_cycle` now has a `fixed` twin (main::t1 and other::t2 both acquire in
the global order a then b) and validates FAIL/PASS in both engines. Codegen emits
one `pub(crate) mod <module>` per CIR module with `crate::`-qualified functions and
module-scoped `Shared` fields. The fixed program codegens, builds, and conforms
**conformant 5/5 runs, coverage 9/9**. Commits ConcIR `79c9440`, and the
ConcPlanVerify benchmark rebuild.

## Not done

- The extraction oracle validated 0/12 in smoke-v2 (extracted CIRs failed
  codegen). Multi-module codegen now exists but the extraction was not re-run
  live; a stronger extraction prompt is still wanted.
- `A3_free`'s program did not call `cir_trace::finish` (all traces missing); the
  free prompt should demand the call.
- Injected `rust/tests/behavior.rs` was substituted by the watchdog-run behaviour
  oracle (deviation D-12).

---

# Round 2026-09-18g — contract strength and complete 12-cell oracle

## F-1..F-6

| item | status | evidence |
| --- | --- | --- |
| F-1 contract under-specification | fixed | `holds_all` + `mutex_exclusive`/`never_holds_all`; 21 buggy contracts carry design intent; `contract-strength-v1`. ConcIR `b829b72`. |
| F-2 extraction oracle | partly | `normalize` drops top-level fields; v2 prompt + parse retry aligned with conform. 12 candidates re-extracted (23 requests): **0 validated**, reasons below. |
| F-3 "terminated ⇒ bug fixed" | fixed | observable `DONE ...` requirement; `behavior_status` enum; wording purged. |
| F-4 SUMMARY hid evidence | fixed | `oracle.miri` shows status counts (`clean 16`/`timeout 16`); `oracle.model` not truncated (reasons section). |
| F-5 channel conformance | fixed | channel ops are attempt-events; `conformance-v4` channel fixed/correct 100%. |
| F-6 A3_free | fixed | free program now uses the injected runtime; `A3_free` produced 66 non-missing traces (all violations — its annotation does not match the CIR). |

## Contract strength (F-1, main result)

`experiments/contract-strength-v1/CONTRACT_STRENGTH.{md,json}`: 14 accepted A3
CIRs replayed offline against a strengthened contract derived from each CIR's own
names (deadlock_free + scope-member completion + `holds_all` per >=2-lock function
+ `var_eq ready`). **12 PASS, 2 FAIL**:

- v2 `partial_deadlock_bystander` accepted v3 → **FAIL** on
  `preserved: main::a holds [main::a, main::b]` (and the symmetric items): the
  "fix" removed the nested critical section — exactly the empty repair F-1 warns
  about.
- smoke-c `partial_deadlock_bystander` accepted v4 → FAIL on
  `preserved: main::bystander completes` (its revision stalled the bystander).

## Conformance v4

`experiments/conformance-v4/CONFORMANCE.{json,md}` — binary
`7cad5515…`, git_rev `b829b72…` (offline part):

| case | traces | conformant | violation | timeout | coverage |
| --- | --- | --- | --- | --- | --- |
| abba / lost_wakeup / permit_leak / scope_bound | 58 | 58 | 0 | 0 | 9/9, 7/7, 5/5, 7/7 |
| cross_module_cycle (fixed) | 58 | 58 | 0 | 0 | 9/9 |
| rendezvous_both_send (fixed) | 58 | 58 | 0 | 0 | 3/3 |
| bounded_backpressure (fixed) | 58 | 58 | 0 | 0 | 5/5 |
| send_while_holding_mutex (fixed) | 58 | 58 | 0 | 0 | 3/3 |
| rmw-zenoh-998 (buggy) | 58 | 27 | 0 | 31 | 9/9 |

`worker_with_payload` skeleton_fill **66/66, coverage 5/5**; `A3_free` built and
produced 66 non-missing traces but **66 violations** (free annotation is
inconsistent with the CIR) — the concrete reason the skeleton arm is needed.

## 12-cell Rust oracle (v2 batch, updated)

Wording is now state-only. `behavior_status`: `no_output` 10, `hang` 1,
`no_build` 1 (the 10 candidates predate the terminal requirement, so they print
no `DONE` line; recorded as `no_output`, not as defects). `oracle.miri`: raw
counts (`clean 16` / `timeout 16`). `oracle.model` is `unverified` for all 12
(reason distribution: invalid/missing sids 6, not a `{cir,rust}` object 4, codegen
1, not run 1). `false_accept` 1 (A0 `partial_deadlock_bystander`, `hang`).

## Extraction (F-2) reason distribution

23 requests across the 12 candidates + 4 retries: 0 `extract_validated`. The
model omits statement `sid`s (`invalid sids` 6), returns no `{cir,rust}` object
(4), or malformed CIR (1). The fixes (drop top-level fields, v2 prompt, parse
retry) are in place; the remaining gap is model compliance, recorded honestly.

## Section 6 (optional): A3 under the strengthened contract

`experiments/contract-strength-v1/a3-partial-rerun/`, 3 requests (K=4): A3 on
`partial_deadlock_bystander` from `repair_input/` under the new contract is
**not accepted** — decisions `explore_fail`, `sid_invalid`, `check_invalid`,
`check_invalid`. The strengthened contract blocks the empty fix, and the model
did not produce a design-preserving repair within 4 rounds.

## Commits

- ConcIR `b829b72` (contract predicates), `a65971f` (channel attempt semantics).
- ConcPlanVerify `98f58c1` (design-intent contracts + strength), `92d3953`
  (extraction fixes), `8bdfabb` (terminal behaviour oracle), `2bded64`
  (conformance-v4, A3_free, A3 rerun).

## Not done

- Extraction oracle validated 0 (model compliance); a stronger prompt or a
  sid-assignment normalisation is needed.
- The v2 candidates cannot satisfy the new terminal requirement (they predate
  it); a live repair-smoke-v3 would give `terminated_ok` numbers.

---

# Round 2026-09-18h — frozen-contract strength and A3_local

## Stop point

Sections 1–3 were completed and committed; **section 3's acceptance metric was
not met** (extraction `extract_validated` = 0 and no per-cell violation
locations, because the failures happen at CIR codegen / Rust build before any
conformance run). Per the "stop at the current section" rule, section 4
(`flash-repair-smoke-v3`) was **not started**. No ConcIR change was needed this
round; Rust tests **238 passed**, binary `6d498c8a…` at `a65971f`.

## G-1 frozen-contract strength (reproducible)

`python/cir_workflow/contract_strength.py` + CLI
`python -m cir_workflow contract-strength --batches ...` regenerated
`experiments/contract-strength-v1/CONTRACT_STRENGTH.{json,md}` from the **frozen**
contracts. The previous derived-contract table is kept as
`CONTRACT_STRENGTH.derived-v0.md` (marked superseded). Every record now carries
`old_contract_sha256`, `frozen_contract_sha256`, `binary_sha256`, `git_rev`.

Result over 14 accepted A3 versions: **v2 `partial_deadlock_bystander` v3 → FAIL
under the frozen contract** on the symmetric
`preserved: main::a holds [main::a, main::b]` / `b` items — the empty fix is
rejected. Some old `abba` accepted versions are `INVALID` under the frozen
contract because they use resource names (`A`/`B`) that the frozen contract does
not declare (recorded, with both contract hashes). Unit tests:
`python/tests/test_contract_strength.py`.

## G-2 A3_local

- `prompts/concir_local_revision_v1.md`; `reply_format="local"` in
  `WholeArtifactRevisionWorkflow`; `_merge_local` merges function bodies and
  resources, preserving everything unmentioned; unreachable new functions are
  rejected.
- `normalize.py` now fills/renames sids and rewrites jump targets (D-17);
  `E208`/`E931` feedback carries the declared base/init example and the
  function's declared params/locals.
- Offline normalizer regression (`experiments/contract-strength-v1/
  NORMALIZATION_REGRESSION.{json,md}`): historical `check_invalid`/`sid_invalid`
  candidates → **3/13 valid** after normalization; the rest carry semantic
  errors.
- Unit tests cover merge-preserves-others, unreachable-new-function rejection, and
  the local undeclared-resource path (`check_invalid` feedback, no crash).

## G-3 extraction v3

Two-fence protocol (`prompts/rust_to_cir_extract_v3.md`) + `parse_extraction`
double-fence handling + expansion of `normalize` (entry FQN, version, function
`kind`). Re-run over the 12 stored candidates (24 requests, at the budget cap):
**0 validated**. Reasons: the extracted CIR still fails `concir-backend` parse for
some shapes, and where it parses the annotated Rust does not build. The v3 prompt
and parser are in place; the remaining gap is model compliance (it does not emit
a codegen-valid CIR plus a matching annotated Rust). This is the section-3
acceptance miss.

## Per-section commits

- `817fd62` contract-strength script + CLI + tests
- `8fc5b7f` A3_local mode, sid fill/rename, E208/E931 feedback, normalizer regression
- `7bdc44d` extraction two-fence v3 + normalization

## Not done

- Section 4 `flash-repair-smoke-v3` (8 tasks × {A0, A1, A2-ml, A3_local,
  A3_whole}) — not started because section 3's acceptance was not met.
- Extraction oracle validated 0 (model compliance).
- The optional `A3_free` extraction (G-5) was not run.

# Round 2026-09-19i — first main table, instrument, dual-track oracle

## I-1..I-9 status (round i)

| item | what was done |
| --- | --- |
| §1 main batch | `flash-repair-smoke-v3` 8 tasks × {A0, A1, A2-ml, A3_local, A3_whole}, 78/200 requests, stop_reason none. Six missing Rust references authored so all 8 tasks run all 5 arms. |
| §2 extraction harness | staged per-cell `extraction_result.json` (`stage`/pointer/stderr/normalizations, `harness_error` counted); `mod cir_trace;` auto-prepend; resource kind/mode, params, FQN, array unwrap/reject. |
| §3 auto-labeling | `concir-instrument` (syn) inserts `cir_trace::ev` at every concurrency call, emits `labels.json` + tagged runtime; `conform --lenient-unlock/--attempt-events`. |
| §4 expert labels | 17 accepted Rust candidates annotated (rubric v1); agreement with the automatic oracle 14/16 = 0.875; two `cycle_3lock` misses (cycle present, accepted by schedule luck). |
| §5 Track D / scale | `detection-v3` (38 records, Lockbud available), `scale-v2` (24 runs) on binary `88c3217d`. |
| §6 A3_free extraction | tool-instrumented label→CIR pipeline; 1/8 validated (`partial_deadlock_bystander`, 28/28 conformant), `harness_error` 0; fixed relative `CIR_TRACE_OUT`. |
| §7 RESULTS | hand-assembled `experiments/RESULTS.md` (superseded by the round-j generator). |

## Rust references (de-leak lint)

Six `rust/{buggy,fixed}.rs` pairs were authored this round (`cross_module_cycle`,
`nested_scope_lock_order`, `notify_one_multi_waiter_wrong_pick`,
`bounded_backpressure_lock_held`, `send_while_holding_mutex`,
`acquire_twice_no_release`). All ten generated references compile with
`cargo build --offline`. The `repair_input/input.rs` de-leak lint (no comments,
no `sN:`/deadlock/bug/fix tokens) reports **0 leak-token lines** for all six.

## Per-section commits (round i)

- ConcPlanVerify: `db20826` batch+refs, `003c300` extraction harness,
  `7a7e634` extract v4 protocol, `ae76710` expert labels, `be45951` Track D/scale,
  `929c169` extraction v4 result, `41032fb` hand-assembled RESULTS.
- ConcIR: `eabad7f` instrument + lenient unlock, `f639ab9` attempt-events +
  runtime tag helpers, `1a83704` instrument doc-comment fix.

## Stop point / Not done (round i)

- Rust-arm `oracle.model` was `inconclusive` for the whole batch (D-19).
- RESULTS was hand-assembled and was inconsistent with SUMMARY (fixed in round j).
- A0/A1 "claims no defect" was misrecorded as `no_build`; A1 accepted one
  unbuilt candidate (fixed in round j §1).
- Binary was not unified (`6d498c8a` for the batch, `88c3217d` for the rest).

# Round 2026-09-20j — making the main table stand up

## I-1..I-9 status (round j)

| item | status |
| --- | --- |
| I-1 A0/A1 reply semantics | §1 done: three-way classifier; A0 claims accepts the buggy input; A1 claims accepts the last build_ok candidate; offline reclass `REPLY_RECLASS.{md,json}`. |
| I-2 false_accept + expert | §2 done: `bug_present` rule (behavior/expert/by_construction/extract/model); `false_accept = accepted AND bug_present`; RESULTS columns `oracle.expert`/`oracle.extract`. |
| I-3 A1 accepted unbuilt | §1 done: A1 invariant asserts a build_ok candidate; `claims_no_issue_unbuilt` otherwise. |
| I-4 RESULTS hand-made | §2 done: `python -m cir_workflow results` generates RESULTS (header has the command + input shas). |
| I-5 HANDOFF Round i | §2 done: Round 2026-09-19i appended. |
| I-6 extraction v5 | §4 done (partial): mandatory-label check, type normalizer, 63 accepted cells; **validated 2 (<3 target) — stop point**. |
| I-7 holds_all hint + case | §5 done: ConcIR template hint + `case-partial-deadlock-v1/CASE.md`. |
| I-8 single binary | §3 done: `BIN_MAIN 4bec943d…`; REBASE_4bec943d (0 changes). |
| I-9 expert provenance | §4: rubric + agent-proxy labels + `HUMAN_REVIEW_QUEUE.md` (left blank). |

## Main batch `flash-repair-main-v1` (§3)

8 tasks × 5 arms × 3 reps, `BIN_MAIN 4bec943d…` (ConcIR `d3d59ed`), 203/300
requests, `stop_reason` none. All 120 cells present. Key numbers (k/n over
reps): A0 24/24 accepted but 12 false accepts (behavior 8, expert 9,
by_construction 3 — overlapping); A2-ml 22/24 accepted, 3 false accepts (all
`cycle_3lock`, expert); A3_local 21/24 accepted, 0 false accepts; A3_whole
20/24.

`cycle_3lock` A0/A2: the 3-lock cycle is present in the as-written program and
accepted by all three reps (Miri clean, Lockbud clean); the expert track catches
it. This is the paper's central "tool-green ≠ correct" evidence.

## Budgets

| run | requests |
| --- | --- |
| main batch (`flash-repair-main-v1`) | 203 / 300 |
| extraction v5 (live) | 98 / 120 |
| case `partial_deadlock` | 17 / 60 |

## Per-section commits (round j)

- ConcIR `d3d59ed` explore: holds_all/never_holds_all repair hint.
- ConcPV: `bebfab8` §1 reply semantics; `e03549d` §2 summary+results+HANDOFF i;
  `a180a07` rebase BIN_MAIN; `5bc2a16`/`107f8c6` §3 main batch; `4c8103f` §4
  extraction v5; `86db1f1` §4 expert labels; `b37388f` §6 Track D; `357306c`
  §5.2 case.

## Stop points / Not done

- §4.3 extraction v5 `validated = 2` (target ≥3): remaining failures are model
  side (CIR omits `kind`, ignores labels, Rust names) plus a few backend panics;
  `harness_error = 0`. Stop point, not a blocker for the main table.
- §4.2 human review: `HUMAN_REVIEW_QUEUE.md` is written and **left blank** for
  the owner (all disagreements + random 4, seed 20260920).
- §8 second-model probe: not run (optional).
- Native schedules are random, so extraction `validated` is not perfectly
  reproducible run to run.

# Round 2026-09-20k — A3 to Rust, per-candidate oracle, reference audit

## J-1..J-7 status

| item | status |
| --- | --- |
| J-1 extraction counting | §1: CELLS.json single source; validated = explore ∧ contract evaluated ∧ verdict∈{PASS,FAIL}; INVALID → contract_invalid; reasons filled (E-code/panic); RESULTS per (task,arm,rep) + stage×reason. |
| J-2 expert per candidate | §4: rubric v2, per-sha labels with `cells` and `evidence`, `design_loss`; unsure 0/49; agreement 95/104. |
| J-3 reference audit | §2: `cycle_3lock` Lockbud DoubleLock = false positive; `partial_deadlock` Miri "detected" = classification bug (path token), fixed with word boundary + `deadlocked` stem; all 8 fixed refs re-run (Miri 16). |
| J-4 aggregation columns | §1: per-rep counts, `llm_ms/tool_ms/verify_ms`, `tokens_per_correct_accept`, A3 decision distribution fixed. |
| J-5 A3_tiered | §5: local→whole escalation, 24 cells, 21 accepted, escalation 3/24, not worse than A3_local. |
| J-6 extraction termination | §6: backend panic → E102; v6 label prompt; validated 0/63 < 5 → `extraction-v6/EXTRACTION_LIMITS.md`, track frozen. |
| J-7 A3 Rust oracle | §3: all 19 unique accepted CIRs codegen (0 holes) + strict conform PASS + behavior + Miri 16; 0 LLM requests. |

## Budgets

| section | requests |
| --- | --- |
| §3 a3-to-rust | 0 / 110 |
| §5 A3_tiered | 27 / 90 |
| §6 extraction v6 | 15 / 40 |
| total | 42 / 260 |

## Binary

- Main batch `BIN_MAIN 4bec943d` (ConcIR `d3d59ed`).
- ConcIR `f42764f` (E102 instead of panic) → new binary `03d343e5`;
  `experiments/REBASE_03d343e5.md` = 0 verdict changes (the change is an error
  path only). The expensive conformance-v4 Miri rebase was not repeated.

## Per-section commits

- ConcIR `f42764f` sem: E102 instead of panic.
- ConcPV `f42c4a0` §1; `ef2c60a` §2; `5ca6ed3` §3; `250ada7` §4; `150e541` §5;
  `88c35fa` §6.

## Not done

- §4 human review: `HUMAN_REVIEW_QUEUE.md` (13 rows) left blank for the owner.
- §5 `A3_tiered` did not help `partial_deadlock_bystander` (2 whole rounds were
  insufficient; A3_whole needed 3).
- conformance-v4 Miri not re-run on the new binary (0-change expectation only).
- A2-ml acceptance was not re-run after the detection-classifier fix (the fix
  only makes detection stricter; noted as a deviation).

# Round 2026-09-2xl — conform recall, generalization, freeze

## K-1..K-6 status

| item | status |
| --- | --- |
| K-1 conform recall | §1 conform-mutation-v1: 86 mutants; M6 (delete ev) recall 1.0 via `violation`; M1 0.545 / M2 0.067 / M4 0.333 all via `timeout`; M3 0.0; M7 control 2/19 false positive. §2 post-edit-conform-v1: 57 edits, conform PASS on all built, `drift_caught_only_by_conform` 0. `CONFORM_GAPS.md` records the stream-level blind spots. |
| K-2 tiered | §3: early escalation (1 local + 3 whole) 3/3 on `partial_deadlock`; original trigger K=6 3/3. The earlier 0/3 was a budget split. |
| K-3 A2 reclass | §6.1: 0 Miri status changes, 0 `tools_green` changes — deviation closed. |
| K-4 generalization | §4: main batch now 10 tasks x 6 arms x 3 reps (55 requests). §5 not done: the endpoint aliases `deepseek-chat`/`deepseek-reasoner` to `deepseek-flash`, so no second model exists. |
| K-5 human review | still blank (owner). |
| K-6 freeze | §6.3 LaTeX tables; §6.4 tags + `FREEZE_MANIFEST.md`. |

## New result that changes a conclusion

Adding `bare_wait_no_predicate` flips the local-vs-whole acceptance ordering:
A3_local 24/30 (0.80) vs A3_whole 26/30 (0.87) — whole now accepts more, because
this task needs a **new predicate statement** that local regeneration cannot
introduce (local 0/3, whole 3/3). Local remains ~5x cheaper (675 vs 3267
tok/correct) and both keep 0 false-accept. So claim (b) is "local is much
cheaper; acceptance depends on whether the fix needs new statements", not "local
dominates".

## Budgets

| section | requests |
| --- | --- |
| §1 mutation | 0 |
| §2 post-edit | 57 / 60 |
| §3 tiered addendum | 15 / 20 |
| §4 10-task extension | 55 / 70 |
| §5 model probe | 0 (not run) |
| total this round | 127 / 220 |

## Binary

`BIN_MAIN 4bec943d` for the main batch; ConcIR `f42764f` (E102) binary
`03d343e5` used for the mutation/post-edit/a3-to-rust offline runs. No
explore/conform semantic change, so the main table was not recomputed.

## Not done

- §5 model-probe-v1: no distinct non-Flash model on the endpoint (aliases return
  `deepseek-flash`); no requests spent.
- K-5 human review queue still blank.
- §1/§2 cover the original 19 programs; the 4 CIRs added by §4 were not mutated
  (conform for them is correct-by-construction only).

# Round 2026-09-2xm — operation-bound conform, second models, human review, freeze 2

## L-1..L-6 status

| item | status |
| --- | --- |
| L-1 conform stream-level | §1/§2: cir_trace v2 emits events from the operation; conform v2 checks (tag,sid,op,resource). Recall v1→v2: M1 0.545→1.0, M2 0.067→0.947, M4 0.333→1.0, M6 1.0, M8 0.8; M5 0.0 (blind spot); M7 21/23 (order-sensitivity, not false positive). |
| L-2 M7 wording | reworded in CONFORM_GAPS v2. |
| L-3 tiered bare_wait | §6: A3_tiered **did** escalate (escalated=true all reps); K=4 fails because the task needs 4 whole rounds (K=6 → 2/3). `TIERED_ADDENDUM.md`. |
| L-4 4 new CIRs | §2/§3 cover all 23. |
| L-5 second model | §5: OpenCode Go, `kimi-k2.7-code` + `glm-5.3-flash`; both reproduce the pattern on 8 tasks. |
| L-6 human review | §0: 17 rows merged (11 candidates); agent vs human 7/7, human vs auto 9/11; evidence map (h) ready. |

## Budgets (round m)

| section | provider | requests |
| --- | --- | --- |
| §0 human merge | — | 0 |
| §1/§2 mutation v2 | — | 0 |
| §3 post-edit v2 | DeepSeek | 64 |
| §4 a3-to-rust v2 | — | 0 |
| §5 model probe | OpenCode Go | 58 |
| §6 tiered addendum | DeepSeek | 18 |

## Binary

- BIN_MAIN `4bec943d` (main table, unchanged).
- BIN_V2 `073129de` (ConcIR `42f7cfa`): cir_trace v2 / conform v2 / codegen v2.
  The change touches conform semantics but not `explore`, so the main table was
  not recomputed; no new `REBASE_*` (explore unchanged).

## Key results

- Operation-bound conform has real recall (M1/M4/M6 = 1.0, M2 0.947, M8 0.8);
  remaining blind spots documented (M5; M7 order-sensitivity).
- Post-edit drift 0 with real edits: the models preserved every sync call.
- Two non-DeepSeek models reproduce the qualitative pattern.

## Not done / stop points

- §5 probe used 8 tasks (SMOKE_V3_TASKS), not 10; `kimi-k2.7-code` needed
  temperature 1; `A0/A2` token counts not recorded.
- §6 used 18 requests (spec ≤12).
- M5 mutation remains a blind spot; instrument v2 (§1.4, free-Rust rewrite to
  wrappers) was not implemented — the mutation/post-edit programs are codegen
  output, so instrument v2 was not on the critical path.

# Round 2026-09-2xn — experiment close-out, paper alignment

## M-1..M-5 status

| item | status |
| --- | --- |
| M-1 probe incomplete | §A1: 10 MAIN tasks x 3 arms for kimi/glm, reusing prior cells; glm 30/30, kimi 27/30 + 3 `not_run` (APIConnectionError) with reasons; A0/A2 tokens recorded; kimi temperature 1 noted; accepted A0/A2 expert-labelled; A3 -> a3-to-rust-v2 (10 PASS / 1 FAIL). |
| M-2 post-edit no-op | §A2: `no_edit` excluded from the denominator; forced retry E2 15/15 real (14 PASS), E3 3/11 real (one moved 5 sync calls, build failed) -> drift 0. |
| M-3 human/auto | §A3: `HUMAN_DISAGREEMENT_EVIDENCE.md` for both candidates; `owner_verdict` blank. |
| M-4 stale text | §A4: RESULTS deviation reworded; commit `4971f650` message/content mismatch noted below. |
| M-5 instrument v2 | not done (optional); scope in the paper stated as codegen products + edits. |

## Note on commit `4971f650`

Its message says "Update CELLS.json and SUMMARY.md …" but the commit actually
contains `HUMAN_REVIEW_QUEUE.md` and old prompt/review files — the user's local
auto-generated message. History is not rewritten; recorded here.

## Budgets (round n)

| section | provider | requests |
| --- | --- | --- |
| §A1 probe completion | OpenCode Go | 26 (kimi 18 + glm 8) |
| §A2 forced edit | DeepSeek | 26 |
| §A3 evidence | — | 0 |

## Freeze 3

- ConcPV tag `experiments-v2-freeze-3`; ConcIR unchanged this round, so no new
  ConcIR tag (`concir-freeze-2` still applies).
- BIN_MAIN `4bec943d` (main table), BIN_V2 `073129de` (conform/a3-to-rust/mutation/post-edit).

## Round n — Not done / stop points

- §A5 `concir-instrument` v2 (free Rust -> wrappers) **not done** (optional).
  The paper's post-verification scope is therefore stated as codegen products
  and their developer edits (`notes/GAP_AUDIT.md`).
- §B2 method section text not rewritten (per instructions); `notes/GAP_AUDIT.md`
  and `notes/METHOD_REWRITE_PLAN.md` carry the plan. Figures are placeholders
  (`notes/FIGURES_TODO.md`).
- The paper repo changes are committed separately in
  `/Users/kevin/paper-review` (commit `2c74d7d`).

---

# Round 2026-09-21o (in progress) — generation benchmark v3, bounded monitor

Repositioned to **generation-first** (requirements -> program); the repair
study is now the second study.  FSE 2027 target, deadline 2026-10-02 AoE.

## Done

| item | status | evidence |
| --- | --- | --- |
| §0 benchmark v3 | **done** | `benchmarks/{TIERS.md,GENERATION_MANIFEST.json,generation_requirements.py}`; 24 tasks across the six families, tiers 8 Simple / 8 Medium / 8 Complex; every frozen contract clause carries `req` tags; `python benchmarks/build_families.py --check-generation` -> 0 errors. Tag `contracts-v3` (commit `b0c346b8`). |
| §0 reference CIR | done | authored `lock-order/two_independent_cycles/fixed.cir.json` (unified pair order), validated FAIL/PASS in both engines. |
| §1 monitor | done | ConcIR `concir-backend monitor` (`src/monitor.rs`) + 8 integration tests; bounded statuses `PASS_bounded/FAIL/not_observed/unmapped/unsupported/deferred`. ConcIR commit `1097b79`. |
| §1 harness | done | `python/cir_workflow/bounded_monitor.py` + `python/tests/test_bounded_monitor.py`; maps clause verdicts onto `req` ids and computes RC/RF. `experiments/rust-oracle-v1/PROTOCOL.md`. |

## Not done / stop points

- **§1 instrument v2** (free Rust -> `cir_trace::sync` wrapper types, sid by
  (resource, kind, order), `resources.json`, failure classification) is **not
  implemented**. Consequence: the bounded oracle can only score artifacts that
  already carry v2 events (codegen products / `G3_concir`), not arbitrary
  LLM-written Rust (`G0`/`G1`/`G2`). This is the first task of the next session.
  The existing call-site annotator does not emit unlock-on-guard-drop, so it
  cannot support `holds_all`/`mutex_exclusive` over time.
- §1 acceptance (fixed.rs -> all non-`[U]` `PASS_bounded`; buggy.rs -> >=1 FAIL
  or hang) was therefore **not run**.
- §2/§3/§4/§5/§6 not started.

## Budgets

| provider | requests |
| --- | --- |
| DeepSeek Flash | 0 |
| OpenCode Go | 0 |

## Binary

- new `concir-backend` (with `monitor`): sha256
  `8480f162ba6c47217a3f8fc0ba0e71d0e1acb85b27efdd23d6354c49e4e9f31f`.
- Rust tests: **246 passed** (238 + 8 monitor). Python: **133 passed**.

## Round 2026-09-21o — update: instrument v2 + §1 acceptance

| item | status | evidence |
| --- | --- | --- |
| instrument v2 | **done** | `concir-instrument --wrappers` (`src/instrument.rs`): free Rust -> `cir_trace::sync` wrapper types, binding-derived resource names, `thread::spawn` events, lazy thread tags, sids by `(resource, op, order)`, `resources.json` + `limitations`. 3 integration tests. |
| monitor alignment | done | `spawn`/`join`/`complete` bound to contract function FQNs; contract `function` names are valid mapping targets. |
| harness | done | `python/cir_workflow/rust_oracle.py` (instrument -> cargo build -> 32 native runs -> monitor -> kind/order auto-mapping -> coverage) + `scripts/run_rust_oracle.py` + tests. |
| §1 acceptance | **run** | `experiments/rust-oracle-v1/{RESULTS.json,SUMMARY.md}`: all 20 references build; **all 10 buggy hang**; **8/10 fixed have every non-`[U]` requirement `PASS_bounded`**. |

- Left at the limit: `condvar/bare_wait_no_predicate` (`var_eq` predicates ->
  `unsupported`, no value events) and `semaphore/acquire_twice_no_release`
  (user-defined semaphore over Mutex+Condvar -> `main::s` `unmapped`).
- Deviation: Miri seeds are wired but not run at 16 seeds for all 20 artifacts;
  `deadlock_free` resolved from native behavior.
- Binary: `concir-backend` `cf4f9e9a…`, `concir-instrument` `5db7765c…`.
- Rust tests **249 passed**; Python **135 passed**.

## Still not done

- §2 generation arms + `flash-gen-main-v1`; §3 probe; §4 repair close-out;
  §5 loom; §6 freeze/evidence-map.

## Round 2026-09-22o2 — generation main batch (§2)

| item | status | evidence |
| --- | --- | --- |
| G arms | done | `scripts/run_gen_main.py`, `python/cir_workflow/generation.py`, `scripts/render_gen_main.py`, tests. |
| `flash-gen-main-v1` | **run** | 288 cells (24 tasks x 4 arms x 3 reps), 629 + 170 requests, no stop. `run-20260922T001631` (Rust arms) + `run-20260922T015338` (G3 re-run after name alignment). |
| tables | done | `experiments/tables/{gen_main,gen_arms,gen_tiers}.tex`, `SUMMARY.md`. |

Key numbers (contract hidden in all arms):

| arm | accept | RC | RF | defect | accepted-with-proof | tokens |
| --- | --- | --- | --- | --- | --- | --- |
| G0_direct | 0.944 | 0.650 | 0.422 | 6 | — | 58k |
| G1_self_iter | 0.931 | 0.656 | 0.460 | 1 | — | 203k |
| G2_tools_iter | 0.639 | 0.639 | 0.468 | 0 | — | 294k |
| G3_concir | 0.625 | 0.776 | **0.504** | 2 | 20/72 | 539k |

- G3 (model-first) has the **highest RF/RC** but ~10x the token cost, and its
  defects are codegen programs that hang while the model passed
  (`condvar/same_cv_different_locks`, 2 reps).
- A **name-alignment** step was required: the model cannot know the contract's
  resource/function FQNs, so candidate names are aligned to the reference by
  kind/declaration order before explore (recorded per cell). Without it G3
  accepted 25/72; with it 45/72.
- Deviation: G2 uses Miri 16 per-seed bounded (15 s), no many-seeds pass (it is
  unbounded on the non-terminating programs); expert-track labels for the
  generation cells are **not run** yet; RESULTS integration and the evidence
  map are §6.

---

# Round 2026-09-22n — generation-first freeze (freeze-4)

Repositioned to generation-first (requirements -> program); repair is the
second study. Sections §0–§3 done; §4 was completed in round n; §5 (loom)
skipped.

| item | status | evidence |
| --- | --- | --- |
| §0 benchmark v3 | done | 24 tasks, tiers 8/8/8, `req`-tagged contracts, `GENERATION_MANIFEST.json`, `--check-generation`. |
| §1 bounded oracle | done | instrument v2 + `monitor`; 10/10 buggy hang, 8/10 fixed all non-`[U]` PASS_bounded. |
| §2 `flash-gen-main-v1` | done | 288 cells; G3 RF 0.504 (best), 539k tok, 2 defects; G2 0 defects. `RESULTS.md` §"Generation (main)". |
| §3 `gen-model-probe-v1` | done | kimi-k3, 72 cells, 112 requests; G3 RF 0.684 with 0 defects. |
| §6 freeze | done | `RESULTS.md` regenerated with generation sections; 12 `tables/*.tex`; `FREEZE_MANIFEST.md` Freeze 4; evidence map v3; tags `experiments-v2-freeze-4`, `concir-freeze-3`. |

## Headline

- Model-first generation (G3) has the **highest requirement satisfaction**
  (RF 0.504 on Flash, 0.684 on kimi-k3) and coverage, but is ~10x the token
  cost of direct generation and is not defect-free: 2 cells passed the model yet
  the codegen Rust hangs (`condvar/same_cv_different_locks`).
- The tool-loop arm (G2) is the only zero-defect Rust arm.
- A **name-alignment** step is required for G3 (the model cannot know the
  contract's resource/function FQNs); without it G3 accepted 25/72, with it
  45/72.

## Budgets (this round)

| provider | requests |
| --- | --- |
| DeepSeek Flash (main batch) | 629 + 170 |
| OpenCode Go (probe) | 112 |

## Not done / stop points

- §5 loom column (optional) not run.
- Expert labels for the generation cells not run (`gen-main` defect uses
  behavior/monitor/conform; expert track pending).
- Part B (paper §B1–§B3) not started.

---

# Round 2026-09-22o (in progress) — G3 v2: LLM code from verified CIR

Architecture correction (REVIEW N-1): ConcIR verifies the CIR; the **LLM**, not
the tool, writes the executable Rust from the verified CIR. Tools post-verify:
`concir-instrument` v2, `conform --op-resource`, `monitor`. Tool `codegen`
becomes the ablation arm.

## Done

| item | status | evidence |
| --- | --- | --- |
| `rust_from_cir_v1.md` | done | `prompts/rust_from_cir_v1.md` (CIR authoritative; requirements supplement). |
| conform op-resource | done | ConcIR `eb3efe4`: `conform --op-resource` matches `(op, resource)` per event (tags/child-tags ignored; scope/spawn/join silent); instrument v2 assigns spawn-order tags. |
| G3 v2 arm | done | `generation.py: run_llmcode_from_cir` (LLM Rust -> instrument -> build -> conform -> monitor -> behavior, conform/monitor feedback, K_code=3). |
| §1.3 smoke | **run** | `experiments/gen-llmcode-smoke-v1/SUMMARY.md`: 45 CIRs, 107 requests, build 45/45, **conform PASS 15/45 (33%)**, accepted-with-proof 15/45; 8 of the 23 codegen-error CIRs now land. |

## Stop point (stop-loss met)

- conform PASS 33% **< 40%**, and the cause is **not** instrument coverage
  (`instrument_limit` empty for almost every cell): it is CIR/code structural
  fidelity (the accepted v1 CIRs often have unusual nested `spawn`/`join`
  graphs that the LLM's Rust does not mirror) and repeated build failures.
- Per the stop-loss rule, **G3 v2 is reported alongside `G3_codegen`, not
  substituted**. The smoke is the v1-CIR diagnostic; §2 (entity names +
  intent-driven requirements) and the §3 rerun are expected to improve the CIRs.
- `run_llmcode_from_cir` returns `accepted_with_proof = conform PASS ∧ monitor no
  FAIL` (model PASS is given by the input CIR).

## Not done

- §2 benchmark v3.1 (entity names, drop heuristic alignment, same_cv intent,
  ConcIR condvar-multi-mutex UNSUPPORTED).
- §3 `flash-gen-main-v2`, §4 probe v2, §5 freeze-5.
- Part B update for the LLM-code architecture.

## Budgets

DeepSeek Flash this round: 2 × 107–115 ≈ **222** (smoke reruns).

---

# Round 2026-09-22o — LLM code from verified CIR (freeze-5)

Architecture correction (REVIEW N-1): ConcIR verifies the CIR; the **LLM**
writes the Rust from the verified CIR; tools post-verify (instrument v2,
`conform --op-resource`, `monitor`). Tool codegen is the ablation arm.

## N-1..N-5 status

| item | status |
| --- | --- |
| N-1 codegen bottleneck | addressed: §1 G3 v2 (LLM code) implemented; codegen kept as `G3_codegen` ablation. |
| N-2 name alignment | addressed: benchmark v3.1 puts **entity names** in the requirements (Entities section); heuristic `name_alignment` removed; CIR-stage INVALID collapses (G3 model PASS 59/72 vs 25/72). |
| N-3 RF dual reporting | done: `RF_all` and `RF_acc` in `tables/gen_main.tex` and SUMMARY. |
| N-4 same_cv anti-pattern | done: requirement rewritten to intent; ConcIR `W1xx condvar_multiple_locks` -> `UNSUPPORTED`. |
| N-5 not done | generation expert labels not run; loom not run; Part B update for the LLM-code architecture pending. |

## §3 flash-gen-main-v2 (312 cells, 680 requests)

| arm | accept | RF_all | RF_acc | defect | awp |
| --- | --- | --- | --- | --- | --- |
| G0_direct | 0.972 | 0.523 | 0.531 | 6 | — |
| G1_self_iter | 0.931 | 0.549 | 0.541 | 3 | — |
| G2_tools_iter | 0.653 | 0.592 | 0.661 | 0 | — |
| G3_concir (v2) | 0.431 | **0.670** | 0.670 | 0 | 31/72 |
| G3_codegen (ablation, rep0) | 0.833 | 0.672 | 0.765 | 0 | 17/24 |

- **CIR stage improved a lot** with entities: 59/72 model PASS (v1: 25/72
  accepted, 46 INVALID);
- **code stage is the bottleneck**: only 31/59 CIR-PASS cells produce Rust that
  passes conform+monitor; `tables/gen_conform_value.tex` shows conform rejected a
  code round in **30/59** cells;
- the ablation still accepts more (0.833 vs 0.417 rep0) — the LLM-code arm is
  reported **alongside**, not substituted (stop-loss met).

## §4 gen-model-probe-v2 (kimi-k3, 100 requests)

G3 v2 RF_all 0.598 vs G0 0.509; G3 accept 0.458, awp 11/24.

## §1.3 smoke

`gen-llmcode-smoke-v1/SUMMARY.md`: 45 CIRs, build 45/45, conform PASS 15/45
(33%); cause is code fidelity, not instrument coverage.

## Budgets

DeepSeek Flash: 680 (main v2) + 107x2 (smoke) ≈ 894. OpenCode Go: 100.

## Not done / stop points

- Generation-cell expert labels; loom.
- Part B (§B1–§B3) not yet updated for the LLM-code architecture (freeze-4
  tables still in the paper).

---

# Round 2026-09-23p — harness fixes + live code stage (freeze-6)

## O-1..O-6

| item | status |
| --- | --- |
| O-1 code-stage failures | tools fixed: single-step condvar wait (suppress the implicit unlock/lock, one `condvar_wait`), `concir_sync::Semaphore` provided + instrumented, name-first thread alignment, var-in-lock convention + violation-kind feedback. |
| O-2 RF_all drift | fixed: `RF_all` counts non-accepted cells as 0; `RF_acc` and `RF_run` added; `_defect` unified. |
| O-3 wording | RF is bounded (monitor) for all arms; `cir_pass` is the exhaustive part. |
| O-4 `_defect` | unified to accepted ∧ (hang ∨ monitor FAIL ∨ Miri detected). |
| O-5 `same_cv` UNSUPPORTED | recorded as a target limitation (ConcIR W1xx). |
| O-6 post-freeze commit | `a0960a93` only toggled `run_g3`/`run_g3_v2` wiring; the main batch ran the version now at HEAD. |

## O-2-corrected metrics

| arm | accept | RF_all | RF_acc | defect | awp |
| --- | --- | --- | --- | --- | --- |
| G0_direct | 0.972 | 0.472 | 0.531 | 6 | — |
| G1_self_iter | 0.931 | 0.390 | 0.541 | 3 | — |
| G2_tools_iter | 0.653 | 0.376 | 0.661 | 0 | — |
| G3_concir (v3 code) | 0.556 | 0.344 | 0.619 | 0 | 40/72 |
| G3_codegen (ablation) | 0.833 | 0.638 | 0.765 | 0 | 17/24 |

- The live code stage (59 verified CIRs, 112 requests) raised G3 accept
  0.431→**0.556** and `accepted_with_proof` 31→**40/72**; the remaining code
  failures are all `extra_op` (real deviations: a lock on a `var`, `main`
  reading shared state), not harness.
- Offline replay of the 28 freeze-5 failures: only **2** are fixed on the old
  programs (they predate `concir_sync`); smoke replay 11/40. The harness-gap
  share is **<50%**, so per the stop-loss the RQ2 claim is "conform rejects real
  deviations and feedback fixes some within K_code", with the ablation alongside.
- G3 accept 0.556 < 0.6 → no further tool iteration; claims to rest on
  `RF_acc`, `awp`, Complex-tier, `accepted_with_proof`.

## Budgets

DeepSeek Flash: 112 (code stage). OpenCode Go: 0 this round.

## Not done

- Semaphore-family G0/G1/G2 not re-run (baseline `unmapped` remains, 表注).
- kimi G3 code stage not re-run (probe v2 carried; predates fixes).
- Generation expert labels; loom (dropped).

---

# Round 2026-09-23q — concir_sync crate, post-join main, v4 code (freeze-7)

## P-1..P-6

| item | status |
| --- | --- |
| P-1 concir_sync double declaration | fixed: `runtime/concir_sync` is a path-dep crate; prompts say "already linked, do not declare `mod concir_sync`"; the harness strips any `mod concir_sync;`/inline `mod concir_sync {..}` and records `harness_notes`. |
| P-2 post-join main read | fixed: `conform --op-resource` allows and counts `post_join_main_ops` once all spawned threads finished; `rust_from_cir_v1` says main may read state after joining. |
| P-3 channel `Mutex<Receiver>` | **partial**: prompt advises moving the `Receiver` into the receiving thread; full wrapper-mutex instrumentation not implemented, so channel `unmapped` persists (37/40 accepted cells). |
| P-4 CIR declared-unused resource | fixed: `check` emits warning `W201`; 3/59 freeze-5 PASS CIRs trigger it (`bounded_backpressure` ×3). PASS/INVALID unchanged; CIR not re-run. |
| P-5 replay design | done: `gen-code-replay-v2` replays the v3-code cells. |
| P-6 queue dedup | `HUMAN_REVIEW_QUEUE_GEN.md` regenerated below (dedup by sha). |

## v3 → v4 (freeze-6 → freeze-7)

- Replay v2 on the v3-code cells: **19 failures → 12 build-fixed + 5 conform-fixed, 2 still failing** (`extra_op`, real deviations); accepted cells: 0 regressions.
- Live `flash-gen-main-v4-code` (140 requests, 86 rerun cells): G3 accept **0.431→0.556→0.750**, `accepted_with_proof` **31→40→54/72**, G3 defect 0. G3 now leads `RF_all` (0.493 vs G0 0.401) and is above G1 on `RF_acc` (0.657 vs 0.534), below G2 (0.683) and the ablation (0.765).
- Semaphore-family G0/G1/G2 re-run with the fixed prompt: their acceptance drops (G0 0.972→0.847) because they can no longer self-implement a semaphore.
- Stop-loss: G3 accept 0.750 ≥ 0.7 → **not triggered**; no failure-reason table needed.
- **Correction recorded**: the round-p claim "remaining failures are all real deviations / harness gap <50% / stop-loss triggered" was based on replaying freeze-5 programs and is void; the correct numbers are above.

## Budgets

DeepSeek Flash: 140 (v4). OpenCode Go: 0.

## Not done

- P-3 full channel-wrapper instrumentation (prompt-only); channel `unmapped` remains.
- kimi G3 code stage (probe v2 carried).
- Generation-cell expert labels; §5 evidence tiering; §6 supplement packaging.

## Round q — §5 cleanup and §6 supplement

- §5: `experiments/EVIDENCE_TIERS.md` (Tier A/B/C), 21 superseded dirs `git mv`-ed to
  `experiments/_archive/` with `WHY_ARCHIVED.md`; active vs legacy scripts split
  (`scripts/README.md`, `scripts/legacy/`); `git clean -fdX -- experiments`
  removed 2466 ignored `target/` dirs, freeing ~12 GB (13 GB → 860 MB).
- §6: `scripts/make_supplement.py` + `make supplement`; produced
  `dist/concplanverify-supplement-experiments-v2-freeze-7.zip` (8.6 MB, 8219
  files) with `SUPPLEMENT_MANIFEST.md` inside; gates all passed (anonymization,
  secret scan, size ≤ 200 MB, self-check re-running `cir_workflow results` to a
  byte-identical `RESULTS.md` and `tables/*.tex`). `dist/` is gitignored.
- Deviation: the secret scan flags secret *values* (`sk-…`, `Bearer …`,
  `KEY=value`), not the environment-variable names the scripts read
  (`os.environ.get("DEEPSEEK_API_KEY")`), which are safe and would otherwise
  exclude every script.

---

# Round 2026-09-23r — baseline parity, channel wrapper, v5 code (freeze-8)

## Q-1..Q-6

| item | status |
| --- | --- |
| Q-1 baseline no `concir_sync` | fixed: one `project_template.cargo_toml` for all arms; unit test compiles `use concir_sync::Semaphore;` for both template names; semaphore baselines re-run (freeze-7's baseline semaphore numbers were harness-limited). |
| Q-2 supplement not anonymized | fixed: private word+secret list moved to gitignored `.supplement-private.json`; anonymization of every text file (incl. `.py/.sh/.toml/.lock`); post-unzip full-tree scan (binary included); `make_supplement.py` excluded; `legacy-cir2cvn`/`real-cases/results`/`raw/` excluded. Package passes all gates (7.2 MB). |
| Q-3 5 failures | `FAILURES.md`: 4× `sem_release` double release (real deviation caused by an API footgun) — `Semaphore::release` removed, `Permit` releases once; `rendezvous_both_send` rep0 real compile failure. |
| Q-4 channel `Mutex<Receiver>` | partial: instrument emits `channel_*` events for direct and mutex-wrapped endpoints and marks wrappers; direct-receiver programs map cleanly. Guard-variable wrappers still leave some `unmapped`. |
| Q-5 probe title / kimi | `gen_results` probe title now reads the data dir; kimi G3 code stage **not re-run** (time); recorded as not done. |
| Q-6 cleanup | reviews untracked (1023 paths); raw tier untracked (103107 paths, 165878→62773 tracked); `RAW_POLICY.md`; `make verify-evidence` green. |

## v4 → v5 (freeze-7 → freeze-8)

| arm | accept | RF_all | RF_acc | defect | awp |
| --- | --- | --- | --- | --- | --- |
| G0_direct | 0.944 | 0.448 | 0.512 | 6 | — |
| G1_self_iter | 0.861 | 0.396 | 0.559 | 3 | — |
| G2_tools_iter | 0.653 | 0.379 | 0.681 | 0 | — |
| G3_concir | **0.806** | **0.555** | 0.689 | 0 | **58/72** |
| G3_codegen | 0.833 | 0.638 | 0.765 | 0 | 17/24 |

- G3 accept 0.750→**0.806**, awp 54→**58/72**, `RF_all` 0.493→**0.555** (highest arm),
  `RF_acc` 0.657→0.689; baselines recovered (G0 accept 0.847→0.944) after fixing
  the shared template. Stop-loss (G3 RF_acc > G0) holds.
- Attribution: Q-1 raised the baseline numbers (G0/G1/G2 semaphore + channel);
  the `Permit` API + channel events raised G3 code acceptance.
- Residual limitation: channel guard-variable wrappers still `unmapped` for some
  old cells; the supplement secret scan flags values, not env-var names.

## Budgets

DeepSeek Flash: 190 (v5). OpenCode Go: 0.

## Not done

- kimi G3 code stage (probe v2 carried); generation-cell expert labels.
- Guard-variable channel-wrapper detection.
- `deepseek-flash-repair-v1` fixture extraction then archive (kept tracked).

## Round r — §6 supplement

- `scripts/make_supplement.py` rewritten: private word/secret list in gitignored
  `.supplement-private.json` (example with placeholders committed); anonymization
  of every text file including `.py/.sh/.toml/.lock`; script absolute defaults
  replaced with `CONCIR_BACKEND`/`CONCIR_REPO` env + relative paths; excludes
  `legacy-cir2cvn/`, `real-cases/results/`, `raw/`, `reviews/`, raw tier and
  `make_supplement.py` itself; post-unzip full-tree scan (binary bytes included).
- Produced `dist/concplanverify-supplement-experiments-v2-freeze-8.zip`
  (**8.9 MB, 8722 files**); gates ok (anonymisation, secrets, size, post-unzip);
  self-check ok (`RESULTS.md` + `tables/*.tex` regenerate byte-identically).
- Deviation kept: the secret patterns match values (`sk-…`, `Bearer …`,
  `KEY=value`), not environment-variable names.
