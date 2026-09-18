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
