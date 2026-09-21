# Rust-arm requirement oracle — bounded trace monitor (v1)

Status: **implemented**. `concir-instrument --wrappers` (v2) rewrites free Rust
onto `cir_trace::sync` wrapper types, the monitor checks the contract against
the observed traces, and `python/cir_workflow/rust_oracle.py` runs the whole
pipeline. See *Acceptance* for the reference results and known limits.

## Free-Rust instrumentation (instrument v2)

`concir-instrument <input.rs> --out <dir> --wrappers` emits `annotated.rs`,
`cir_trace.rs`, and `resources.json`. The transform:

- replaces `std::sync::{Mutex, Condvar}` with wrappers whose `lock` and
  `Guard::drop` (and `wait`/`notify_*`) emit operation-bound events, so
  guard-drop unlocks are observed;
- names each constructor from its binding (`let mtx_a = Arc::new(Mutex::new(())`
  -> `mtx_a_mutex0`), so `Arc::clone`d handles share one name; tuple
  components get `_mutex0`/`_condvar0` suffixes;
- rewrites `thread::spawn(..)` to `cir_trace::spawn(name, ..)` (completion is
  witnessed by the run finishing);
- derives thread tags lazily (`t0` for main) and sids by
  `(resource, op, occurrence)`;
- lists what it cannot cover (`RwLock`, `Barrier`, atomics, async) in
  `limitations` instead of failing silently.

## Two oracles, two words

| arm | oracle | strength |
| --- | --- | --- |
| `G3_concir` (model) | `concir-backend explore` / `conform` on the CIR + codegen | **exhaustive** over the bounded model |
| `G0/G1/G2` (free Rust) | `concir-instrument` traces + `concir-backend monitor` | **bounded** over observed runs |

The paper must not use one word for both. A model verdict is `PASS`/`FAIL`; a
trace verdict is `PASS_bounded`/`FAIL`/`not_observed`/`unmapped`/`unsupported`.

## Trace format

One JSON object per line, as emitted by the `cir_trace` runtime:

```json
{"t":"t1","sid":"L3","op":"mutex_lock","r":"mtx_a"}
```

`op` is one of `mutex_lock`, `mutex_unlock`, `condvar_wait`,
`condvar_notify`, `condvar_notify_all`, `sem_acquire`, `sem_release`,
`channel_send`, `channel_recv`, `spawn`, `scope`, `join`, `complete`. `r` is the
runtime resource name (a variable name), `sid` a per-run synthetic step id.

Trace sources: native execution **N = 32** runs plus Miri **16** seeds, each run
writing one trace file (one `cir_trace` stream).

## Resource alignment

`concir-backend monitor --resources resources.json` aligns runtime names to
contract FQNs by exact match, then `main::<name>`, then `::`-suffix. A name that
aligns to no contract resource is listed in `unmapped_resources` and every
clause that needs it is `unmapped`. `--mapping mapping.json`
(`{ "<rust_name>": "<contract_fqn>" }`) is the documented manual escape hatch,
recorded with provenance by the harness.

## Clause semantics (bounded)

- `safety`, `never_holds_all`, `unreachable` — must hold in **every** observed
  state of every run; a single violation is `FAIL`.
- `reachability`, `always_reachable`, preserved `reachable` — `PASS_bounded` if
  witnessed in at least one observed state, else `not_observed`.
- `deadlock_free` — the monitor returns `deferred`; the harness resolves it from
  behavior (`DONE` terminal before timeout) and Miri.
- `function_completed` — bounded: a spawned child counts complete when its
  `join` (or an explicit `complete`) is recorded; a finished trace implies
  `main` returned.
- `var_eq` / `var_cmp` — `unsupported` (the stream carries no values).

## Requirement coverage

Contract clauses carry a `req: ["R3","R5"]` tag (benchmark v3). Per requirement:

- `FAIL` if any covering clause failed;
- otherwise the highest-priority covering status
  (`FAIL > unmapped > unsupported > not_observed > deferred > PASS_bounded`);
- requirements with no covering clause are `unverifiable` (`[U]` in
  `REQUIREMENTS.md`).

**RC** = decidable requirements / total (`PASS_bounded`, `FAIL`, `not_observed`
are decidable). **RF** = `PASS_bounded` requirements / total. Both are reported
per tier and overall.

## Command

```bash
concir-backend monitor --contract <contract.json> \
  [--resources <resources.json>] --traces <dir> [--mapping <mapping.json>]
```

Exit 0 unless a required-to-hold clause `FAIL`s. Python wrapper:
`cir_workflow.bounded_monitor.run_monitor` / `.coverage`.

## Acceptance (10 fixed + 10 buggy references)

Run: `PYTHONPATH=python python3 scripts/run_rust_oracle.py`
(-> `RESULTS.json`, `SUMMARY.md`).

- All 20 artifacts instrument, build, and run (32 native runs each).
- All 10 `buggy.rs` references **hang** (deadlock), so the defect is detected.
- 8/10 `fixed.rs` references have every non-`[U]` requirement `PASS_bounded`.
  The two exceptions are instrument/alignment limits, not program defects:
  `condvar/bare_wait_no_predicate` (its flag clauses are `var_eq`; a free-Rust
  trace carries no values -> `unsupported`) and
  `semaphore/acquire_twice_no_release` (the reference implements its own
  semaphore over `Mutex`+`Condvar`, so no `Semaphore` resource aligns to
  `main::s` -> `unmapped`).

## Stop points

- `var_eq`/`var_cmp` predicates need value events; a future instrument could
  hook atomics/shared writes.
- User-defined semaphores are not recognized; only std primitives are wrapped.
- Miri seeds are wired (`rust_oracle.run_miri`) but were not run at 16 seeds
  for all 20 artifacts this round; `deadlock_free` is resolved from native
  behavior. This is a deviation for the batch to record.
