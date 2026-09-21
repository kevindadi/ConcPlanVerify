# Rust-arm requirement oracle — bounded trace monitor (v1)

Status: **partial**. The monitor and the Python harness are implemented and
tested; the free-Rust instrumenter (instrument v2, wrapper types) is **not**
implemented yet (see *Stop point*). Until then the trace producer for free Rust
is the existing call-site annotator, which emits `lock`/`wait`/`notify`/`send`/
`recv`/`join`/`spawn` events but not unlock-on-guard-drop.

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

## Stop point

`concir-instrument` v2 (free Rust -> `cir_trace::sync` wrapper types, sid by
(resource, kind, order), `resources.json`, failure classification) is **not
implemented**. Therefore the bounded oracle can, today, only score artifacts
that already carry v2 events (codegen products, `G3_concir`), not arbitrary
LLM-written Rust (`G0/G1/G2`). This is the first task of the next session.
