# EXPERIMENTS V2 — FROZEN PROTOCOL (2026-09-18)

Status: **FROZEN for offline execution.** The live DeepSeek Flash batch is not
run until the user confirms this document. All parameters below are frozen; any
deviation must be recorded as a deviation in the final handoff.

This protocol implements Section B of the round plan. It is deliberately
descriptive: it fixes arms, budgets, tools, oracle rules and output schemas so
that the offline scripts and the live batch are the same harness with a different
provider.

## B0. Shared-input design

- All generation arms share the **same task input**: the natural-language
  `spec.md` for a task, the same model (`deepseek-flash`), temperature 0,
  `max_tokens=4096`, thinking disabled, the same per-arm round cap `K=4`, and the
  same overall budget.
- Only the **verification/feedback source** differs between arms.
- Each arm's "accept" is decided by that arm's own decider. Final correctness is
  decided by a **terminal oracle independent of every arm** (B4). The two are
  reported separately so `false_accept` can be computed.
- Consumption is measured, never estimated: HTTP requests, prompt/completion/
  total tokens, LLM wall-clock, tool wall-clock, iteration rounds, and the round
  in which the first correct artifact appears. Missing provider usage is recorded
  as `null`, never as zero.
- LLM iteration inside an arm means "the LLM rewrites the whole artifact" (Rust
  source or CIR). Our arms additionally keep `external_single_patch` as a
  separate mode (`A3p_ours_patch`); it is never mixed into whole-artifact
  revision counts.
- Model rules (unchanged from previous rounds): provider `deepseek`, model
  `deepseek-flash`, thinking disabled, `max_tokens=4096`, `temperature=0`,
  timeout ≤ 90 s, SDK `max_retries=0`, at most one transient retry,
  stop-the-batch on a response whose model != requested model, no Pro, no
  aliases, no fallback. The key is read only from `ConcPlanVerify/.env` and is
  never written to any artifact. The model name is a parameter of the harness.

## B1. Task set

Canonical pattern set from the paper (Table 1), mapped to the legacy reference
programs under `benchmarks/legacy-cir2cvn/benchmarks/rust/`:

| id | paper pattern | legacy program | defect type | expected |
| --- | --- | --- | --- | --- |
| P1 | Two-mutex deadlock | `mutex_deadlock` | Deadlock | bug |
| P2 | Condvar signal loss | `signal_loss` | SignalLoss | bug |
| P3 | Channel + mutex DL | `channel_deadlock` | Deadlock/ChannelBlock | bug |
| P4 | Three-lock circular | `three_way_deadlock` | Deadlock | bug |
| P5 | Partial deadlock (bystander) | `partial_deadlock` | GoalUnreachable | bug (goal layer) |
| P6 | Dual condvar cross | `dual_condvar` | SignalLoss/Deadlock | bug |
| P7 | Semaphore throttle | `semaphore_throttle` | none | bug-free baseline |
| P8 | CAS contention | `cas_race` | none | bug-free baseline |
| P9 | FnSummary propagation | `fn_summary_prop` | none | bug-free baseline |

Scale set S (only for B5): `deep_lock_chain_4x3`,
`scale_lock_chain_5x3_buggy`, `scale_lock_chain_6x3`, `scale_branch_fan_4x2`,
plus `P4` variants with `k=2..5` threads.

Real cases: `real-cases/rmw-zenoh-998` (verifiable) and
`real-cases/dashmap-369` (RwLock, expected `UNSUPPORTED`, kept as-is).

Each task directory `benchmarks/patterns/P<N>/` contains:

- `spec.md` — natural-language design intent, **not** stating where the bug is;
- `buggy.rs`, `fixed.rs` — Rust reference programs (when both exist in legacy);
- `buggy.cir.json`, `fixed.cir.json` — current modular-CIR ground truth;
- `contract.json` — `deadlock_free` plus per-task `function_completed`
  preserved goals; P2 adds `var_eq ready=true`; P5 uses
  `reachable`/`always_reachable` so the defect is exposed without a global
  deadlock;
- `ground_truth.json` — defect type, resources/statements involved, expected
  repair class;
- `behavior/` — a `cargo test` behavioral check (completes within a timeout and
  produces the expected result, e.g. `ready=true`, all workers finished).

Rust programs must `cargo build`. `fixed.rs` must pass the behavioral test;
`buggy.rs` is allowed to time out or fail. Every file's sha256 is recorded in
`benchmarks/MANIFEST.json`; once frozen, live runs reference only hashes.

Authoring status is recorded per task in the manifest (`status`: `ready` |
`to_author`). `to_author` tasks are excluded from the live batch until their
CIR/behavior files exist and validate; the exclusion is reported.

## B2. Detection capability (Track D, no LLM)

Run each detector on `buggy.rs` / `fixed.rs` (Rust tools) and
`buggy.cir.json` / `fixed.cir.json` (ConcIR), reporting detection / miss / false
positive, bug kind, and wall-clock.

- **lockbud**: not currently installed. Follow its README with a compatible
  nightly, pin the commit/version in the manifest. If installation fails, record
  `unavailable` and stop that arm; no substitute tool is used.
- **miri**: use `cargo miri run`. Frozen seed / preemption-rate table (`N=5`):
  `(seed=0, preemption_rate=0.01)`, `(1, 0.05)`, `(2, 0.10)`, `(3, 0.20)`,
  `(4, 0.50)`. Record whether a deadlock/data race is reported, the first seed
  that detects it, and total wall-clock. Miri is dynamic: a miss is **not**
  "no bug"; the report must say so.
- **ConcIR**: `explore` (`petri` and `interp` differential) on the ground-truth
  CIR; record outcome, `complete`, states, transitions, wall-clock and (if B7
  shipped) refinement labels. This arm consumes human-written CIR, not source;
  the comparison table must state the input-form difference. This is a
  model-layer-exhaustive vs source-layer-static/dynamic comparison, **not** a
  same-input fair comparison.

Output: `experiments/detection-v1/DETECTION.md` + `DETECTION.json`.

## B3. Generation + iteration arms (Track G, K=4 frozen)

| arm id | feedback source | accept rule |
| --- | --- | --- |
| `A0_direct` | none; one shot | structural/static validity only (build for Rust; `check` for CIR) |
| `A1_self_iter` | LLM self-review only ("find and fix concurrency defects, output the full program") | LLM reports "no problems" or K rounds |
| `A2_tools_iter` | `cargo build` + lockbud + miri (frozen B2 config), raw diagnostics truncated to 8 KiB | all tools green or K rounds |
| `A3_ours_revision` | modular CIR `check`/`support`/`explore` structured diagnostics + fidelity, whole-CIR revision | complete `PASS` + preserved (fidelity reported separately) or K rounds |
| `A3p_ours_patch` | existing `repair-context → evaluate-patch → replay` single patch on the first statically valid FAIL CIR | Rust accepts and replay confirms; other patterns `not_applicable` |
| `A3_tool_repair` | deterministic backend strategy `c`, zero LLM cost (cost reference) | `repaired`/`already_satisfied` + replay |

Per arm × task record: `arm`, `task`, per-round sequence (request hash, tokens,
LLM wall, tool wall, raw tool-output hash, arm decision), `arm_accepted`,
`accepted_round`, total consumption. Terminal verdict in B4.

Python side: new `revision_workflow` supporting whole-artifact LLM revision
before and after freeze, label `repair_mode=llm_revision`, every CIR version
archived and hashed, acceptance still by full Rust verification. Scripted offline
tests cover `FAIL → revise → PASS`, `revision introduces a static error →
feedback → fix`, and `K rounds exhausted`.

Rust side: `rust_arm` writes LLM output into an isolated temporary cargo project
(fixed `Cargo.toml`, std only); `cargo build`/`cargo test`/lockbud/miri run as
subprocesses with timeouts and no network; raw stdout/stderr archived; a build
failure is itself a feedback round.

## B4. Terminal oracle (independent of every arm)

For each arm's final artifact:

- **Rust artifact**: (i) `cargo build`; (ii) B1 behavioral test (completes within
  timeout + expected result); (iii) lockbud + miri frozen config; (iv)
  human/rule comparison against ground truth for whether the defect is still
  present (decision basis recorded). Four separate booleans; never a single
  fused score; "not detected" is never written as "correct".
- **CIR artifact**: full Rust verification (`PASS` + preserved) + fidelity; the
  final CIR's `explore` counterexample/blocked facts compared against
  ground-truth statements.

Derived, reported separately:

- `false_accept = arm_accepted ∧ oracle_bug_present`
- `behavior_loss = arm_accepted ∧ behavior_test_fail` (Rust arms)
- `conservative_reject = ¬arm_accepted ∧ oracle_clean`

## B5. Scale and cost (no LLM)

S group + P4 variants: vary thread count `k=2..5`, lock-chain length and bounds;
record states/transitions/wall-clock; mark the UNKNOWN boundary. ConcIR
per-call wall-clock is reported next to miri/lockbud per-run wall-clock. Output:
`experiments/scale-v1/`.

## B6. Ablations (A3 only; offline scripted first, live in the same batch)

- `A3_nodiag`: feedback is only `PASS`/`FAIL` (no structured diagnostics).
- `A3_nopreserved`: contract drops `preserved`; accept only `deadlock_free`;
  count terminal-verified behavior-loss accepts.
- `A3_nofidelity`: no fidelity check; count "model quietly fixed the defect" that
  would be miscounted as success.

## B7. Rust refinement labels (optional)

If required by B2: add **post-hoc** refinement labels `signal_loss` /
`channel_block` to confirmed-deadlock diagnostics using existing blocked facts
and witness only; do not change the outcome; do not emit labels when not a
deadlock; replay covers the field and stays compatible with old artifacts. If
shipped, add P2/P3/P6 regressions and update `doc/`. If not shipped, say so in
the handoff. Default this round: **not shipped** unless B2 shows it is needed.

## B8. Budget and batches (freeze before running)

- Offline: all harnesses (Track D, Rust subprocess arms, revision workflow,
  ablation switches, oracle, metric aggregation) run with scripted providers; no
  network, no key; all existing tests stay green.
- Live batch `experiments/flash-arms-v1/`: tasks = P1–P9 + `rmw-zenoh-998`
  (10 tasks, `to_author` tasks excluded and reported); arms = `A0`, `A1`, `A2`,
  `A3`, `A3p`, `A3_nodiag`, `A3_nopreserved` (`A3_nofidelity` is pure statistics,
  no extra requests). Caps: `K=4`; **total HTTP ≤ 260**, **wall-clock ≤ 4 h**;
  a single shared `budget.json` across the batch; a restart never resets it.
  Execute arm-by-arm; if any stage stops the batch, deliver the data collected.
- Records match previous rounds: sanitized messages, prompt sha256,
  requested/response model, request id, `finish_reason`, `usage` or null, timing,
  round, feedback, full tool argv/exit/stdout/stderr and hashes. Before delivery,
  scan every evidence file to confirm the key string is absent.
- Summary script produces `SUMMARY.md`/`SUMMARY.json`: per arm × task
  accept/oracle/rounds/tokens/wall table; per-arm aggregates (accept rate,
  false-accept rate, behavior-loss rate, mean rounds, mean tokens, mean wall);
  and the A3-vs-A2 cost decomposition (LLM time vs tool time).

## Frozen constants

| constant | value |
| --- | --- |
| model | `deepseek-flash` (provider `deepseek`) |
| temperature / max_tokens / timeout | 0 / 4096 / 90 s |
| thinking | disabled |
| SDK retries | 0, plus ≤ 1 transient retry |
| K | 4 |
| miri combos | 5 (seed, preemption_rate) pairs above |
| diagnostic truncation | 8192 bytes |
| live HTTP cap | 260 |
| live wall cap | 14400 s |
| batch budget file | `experiments/flash-arms-v1/budget.json` |

## Revision 2026-09-18b — capability families and deviations

The task set is rebuilt from the **current ConcIR capability matrix**, not the
paper's Table 1. Old `cir2cvn` capabilities (FnSummary, three-valued guards,
RwLock) no longer exist; the current backend adds modules, `scope`/`bound`,
bounded channels, a precise condvar wait-set, counting semaphores, bounded data
domains and `safety`/`reachability`/`always_reachable`/`unreachable` properties
that the paper did not cover. Tasks live in `benchmarks/families/<family>/<case>/`
and the retired patterns are kept under `benchmarks/legacy-paper-patterns/`
with `status: legacy`.

Design change: **the target properties are written and frozen by a human**, not
derived by the LLM. This differs from the paper's "LLM can derive business
goals" wording and is intentional.

Registered deviations (must be repeated in the handoff):

- **D-1 (Miri sample size).** Each Miri run is ~0.2-0.6 s, so the frozen 5
  single-seed combinations are insufficient to support a miss conclusion. The
  harness now also runs one `-Zmiri-many-seeds=0..64` pass (falling back to a
  64-seed loop when the flag is unsupported). The 5 frozen combinations are
  still reported separately. Miri remains dynamic: a miss is never safety.
- **D-2 (task set).** P1-P9 are retired to `legacy`; experiments use the
  capability families. Boundary cases (`UNSUPPORTED`, `UNKNOWN`) are negative
  controls and have no Rust reference or behavior test.
- **D-4 (Lockbud available).** Lockbud is built under `tools/lockbud` (commit
  `cc78cb7`, `nightly-2026-02-07`) and run as a `RUSTC_WRAPPER` with
  `-k deadlock -l cir_arm_probe`; results are parsed from `bug_kind` JSON
  records, not from text. Its reports are "possibly" over-approximations.
- **D-3 (behavior tests).** Rust behavior tests are real (`rust/tests/`) where
  authored; a zero-test project is `behavior_test_ok = null` with reason
  `no_tests`, never `true`. Cases without an authored behavior test report
  `null` for that oracle field.

## Revision 2026-09-18d — repair-type main experiment + consistency layer

- **D-5 (repair-type main experiment).** The multi-arm comparison is repair-type,
  not generation-type: each task supplies a defective program guaranteed by
  construction (Rust for A0/A1/A2, the corresponding CIR for A3) plus the frozen
  requirement. Generation-type results are a supplementary table only. Smoke
  protocol: `experiments/flash-repair-smoke-v1/PROTOCOL.md` (task list and arms).
- **D-6 (code-level conformance).** ConcIR adds `codegen` (sid-annotated, std-only
  Rust skeleton with `// HOLE` placeholders and a `cir_trace` runtime) and
  `conform` (replay a trace against the reference interpreter). Events are
  emitted after a **completing** step for lock/acquire/condvar-wait/channel
  (blocking attempt) and when the statement is reached for
  unlock/notify/release/scope/spawn/join. `conform` treats a blocking attempt and
  a blocked resume as silent where appropriate, carries a frontier of candidate
  model states (nondeterministic bindings), and assigns child tags during silent
  spawn/scope steps. A conformant trace means "every observed execution is a
  model execution"; it is not a correctness proof.
- **A2 acceptance tiers.** `A2_tools_iter_m` = build + Miri (5 frozen seeds);
  `A2_tools_iter_ml` = + Lockbud. Per round the harness records `build_ok`,
  `test_ok`, `miri_green`, `lockbud_green`, `lockbud_detected`.
- **C-1..C-5 fixes.** A1 parses `NO_ISSUES`; A3 treats a `check` usage/protocol
  error as schema feedback (not a tool error); outputs must contain `fn main`;
  SUMMARY keys match arm ids; `.gitignore` re-includes `experiments/**/*.txt`.

## Threats to validity (must appear in the final handoff)

- Miri is dynamic and non-exhaustive: a miss is not proof of absence.
- The ConcIR detection arm consumes human-written CIR; other arms consume Rust
  source. The comparison is capability-level, not same-input.
- Single model (Flash) and small sample size; this is a feasibility + first-data
  study, not an effect-size study.
- `to_author` tasks are reported and excluded rather than faked.
