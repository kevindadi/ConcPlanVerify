---
name: concir-verify
description: >-
  Verify, generate, and repair concurrent-program models written in ConcIR
  (Concurrency Intermediate Representation) using the cir2cvn -> CVN deadlock /
  signal-loss / channel-block analysis. Use when the user asks to model a
  concurrent system, check a model for deadlocks / signal loss / unreachable
  business goals, or fix a buggy concurrency model. Trigger keywords: ConcIR,
  CVN, cir2cvn, deadlock, signal loss, condvar, channel, concurrent model,
  deadlock repair, verify concurrency.
license: MIT
---

# ConcIR Verification (cir2cvn)

A two-layer toolchain for verifying concurrent program models:

```
ConcIR JSON  --(translate)-->  CVN Petri net  --(state-space explore)-->  verdict
```

- **Deterministic engine**: the Rust binary `cir2cvn` (validate / analyze /
  goals) and the Python CLI `cir_workflow` (generate / repair / plan / merge).
- **You (the LLM) decide**: this skill is the decision layer. Run the tools,
  read the structured JSON verdicts, and drive the generate -> validate ->
  analyze -> repair loop yourself.

This skill is tied to the **ConcPlanVerify** repository, which contains the
toolchain. Work from inside that repo (or point the scripts at it via
`CONCPLANVERIFY_ROOT`). A short ConcIR syntax reminder is at the end.

## 1. Setup (one time)

```bash
bash skills/concir-verify/scripts/build.sh
```

This builds `target/release/cir2cvn` and creates the Python venv
`python/.venv` with its dependencies. It is idempotent.

## 2. Tool invocation

| Task | Command |
| ---- | ------- |
| Static validation (58 rules) | `scripts/validate.sh <file.json \| ->` |
| Full verification (analyze) | `scripts/analyze.sh <file.json \| ->` |
| Verification with goals | `scripts/goals.sh <file.json \| ->` |
| LLM generation (DeepSeek/Qwen) | `python -m cir_workflow generate --provider deepseek --requirements "..."` |
| LLM repair loop | `python -m cir_workflow repair [--provider deepseek] <file.json>` |
| Modular plan + merge | `python -m cir_workflow plan --requirements "..."` then `python -m cir_workflow merge bundle.json` |

Python commands run as: `PYTHONPATH=python python/.venv/bin/python -m cir_workflow ...`
from the repo root. The scripts accept `-` for stdin.

## 3. Reading a verdict (analyze / goals output)

The tools print one JSON object. Fields you act on:

- `status`: `verified_safe | verified_unsafe | goals_unmet | invalid_model |
  translation_failed | analysis_incomplete`
- `valid`: ConcIR static validation passed?
- `diagnostics[]`: validation/translation errors, each `{code, severity, message,
  path, fix_hint}`.
- `bugs[]`: concurrency bugs, each `{kind, summary, trace[], involved_resources[],
  involved_functions[], final_marking_summary, repair_hint, cir_slice[]}`.
  `kind` is one of `Deadlock`, `SignalLoss`, `ChannelBlock`, `DeadTransition`.
- `unmet_goals[]`: unreachable business goals `{goal, reason}`.
- `goal_warnings[]`: goal translation warnings (e.g. "too weak", unknown key).
- `state_count`, `places`, `transitions`, `analysis_complete`,
  `analysis_error`, `timings`.

See `reference/verification-contract.md` for the exact shapes and error-code
families (E0xx validation, T0xx translation, V3xx analysis).

## 4. Decision workflow (the loop you drive)

1. **Obtain ConcIR**: user provides a file, you generate one from requirements
   (via `cir_workflow generate` or by hand), or you repair an existing one.
2. **Validate** (`--validate`). If `invalid`: read `diagnostics[].code` +
   `fix_hint`, fix the ConcIR, re-validate. Repeat until `valid`.
3. **Analyze** (`--analyze`). Branch on `status`:
   - `verified_safe` -> done. Report the metrics (states/places/transitions).
   - `verified_unsafe` -> read `bugs[]`; classify and repair (below), then
     re-run validate + analyze. At most 5 rounds.
   - `goals_unmet` -> read `unmet_goals[]` + `goal_warnings[]`; fix the model so
     the goals become reachable, re-verify.
   - `invalid_model` / `translation_failed` / `analysis_incomplete` -> fix the
     reported errors (translation errors are T0xx, analysis errors V3xx), re-run.
4. **Repair by bug kind**:
   - `Deadlock` -> enforce a uniform lock ordering across threads; participants
     are in `kind.Deadlock.participants[]` (`holding`/`waiting_for`).
   - `SignalLoss` -> the notify fired before the waiter entered `wait`; wrap the
     wait in a while-loop over the predicate variable.
   - `ChannelBlock` -> send/recv have no counterpart; do not hold a mutex across
     a blocking `recv`/`send`, and ensure a matching producer/consumer exists.
   - `DeadTransition` -> statement is unreachable on every interleaving: a
     branch guard is statically falsified, a required `spawn`/`notify`/`send` is
     missing, or a `next` target bypasses the statement.
   - Use `repair_hint` and `cir_slice` to pinpoint the statement to change.
5. **Re-verify** after every change; stop when `verified_safe` (and goals met)
   or the round budget is exhausted (then report the last verdict honestly).

## 5. Guardrails

- API keys live in the repo-root `.env` (`DEEPSEEK_API_KEY`,
  `DASHSCOPE_API_KEY`). **Never print, log, or commit them.**
- On repair, output the **complete** revised ConcIR JSON — do not drop any
  resource, protection entry, function, or goal.
- ConcIR JSON must round-trip: unknown fields are rejected (strict schema).
- If analysis times out or the state space explodes, check `analysis_error`
  and consider bounding Int variables (a `{"Int": [lo, hi]}` base on Var
  resources) rather than removing spawns.

## 6. ConcIR syntax reminder

```jsonc
{
  "program": "name",
  "resources": [ /* kind: "sync"|"var"; type: Mutex|RwLock|Semaphore|Channel|Condvar|Var|Atomic */
    {"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"}
  ],
  "protection": [ {"var": "x", "lock": "mtx"} ],
  "functions": [
    {
      "name": "main", "kind": "normal",
      "body": [
        {"sid": "s1", "op": ["res_op", "mtx", "lock"], "transfer": ["next", "s2"]},
        {"sid": "s2", "op": ["spawn", "worker"], "transfer": ["next", "s3"]},
        {"sid": "s3", "op": ["join", "worker"], "transfer": ["next", "s4"]},
        {"sid": "s4", "op": ["res_op", "cv", "wait", "mtx"], "transfer": ["next", "s5"]},
        {"sid": "s5", "op": ["res_op", "ch", "send", "msg"], "transfer": ["next", "s6"]},
        {"sid": "s6", "op": ["call", "helper"], "transfer": ["next", "s7"]},
        {"sid": "s7", "op": "return", "transfer": "return"}
      ]
    }
  ],
  "entry": "main",
  "goals": [ {"id": "g1", "marking": {"main.s7": 1}, "variables": {"count": 5}} ]
}
```

- `op` shapes: `["res_op", resource, action, args...]` (actions: lock, drop,
  read, write, send, recv, acquire, release, load, store, cas, wait, notify,
  notify_all), `["spawn"|"spawn_async"|"join"|"await", fn]`,
  `["call", fn, outvar?, args...]`, `["return", expr?]`, `"return"`, `"nop"`.
- `transfer` shapes: `["next", sid]`, `["branch", cond, true_sid, false_sid]`,
  `["switch", var, {label: sid}]`, `"return"`.
- sids look like `s1`, `s2`, ... (alphanumeric, may contain `_`).
- `modeled: true` params/returns enter the net; unmodeled ones are codegen-only.

## 7. More detail

See `reference/verification-contract.md` for the JSON output contract and
error-code catalog. Example programs: `tests/fixtures/*.json` and
`tests/e2e/*/buggy.json|fixed.json`.
