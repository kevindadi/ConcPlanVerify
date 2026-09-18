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
