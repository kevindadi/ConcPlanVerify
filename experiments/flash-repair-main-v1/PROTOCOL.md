# flash-repair-main-v1 — PROTOCOL (frozen 2026-09-19j)

Main multi-arm repair batch, three repeats on a single binary. This is the
paper-facing table.

## Tasks and arms

8 hard tasks (unchanged from `flash-repair-smoke-v3`):
partial_deadlock_bystander, cross_module_cycle, cycle_3lock,
nested_scope_lock_order, notify_one_multi_waiter_wrong_pick,
bounded_backpressure_lock_held, send_while_holding_mutex,
acquire_twice_no_release.

Arms: `A0_direct`, `A1_self_iter`, `A2_tools_iter_ml` (build + Miri + Lockbud),
`A3_local` (local regeneration), `A3_whole` (whole-artifact resend). K=4.
Reps: `rep=0,1,2`; each rep is a directory level `rep-N/` and each request
carries its rep in the arm record.

Task order is rep-outer, task-inner so `rep=0` completes first. If the budget
runs out in reps 1–2 the arm priority is A0, A2-ml, A3_local, then A1, A3_whole;
unrun cells are recorded as `not_run`, never missing.

## Model and budgets

`deepseek-flash`, thinking disabled, temperature 0, `max_tokens=4096`, timeout
90 s, SDK retries 0 plus at most one transient. Key from `.env` only. Shared
budget: **<=300 requests**, wall <=3 h; on exhaustion `stop_reason=budget` and
completed cells are kept. Miri terminal oracle: 16 seeds x 8 s.

## Reply semantics (three-way classifier)

A0/A1 replies are classified as:
- `program`: contains a complete Rust program (fenced or raw, `fn main` present);
- `claims_no_issue`: the `NO_ISSUES` sentinel, or fence-free prose matching the
  word list below (case-insensitive);
- `other`: neither; one format retry is issued (counted), then `format_error`.

Word list (initial): `no issue`, `no defect`, `no bug`, `is correct`,
`already correct`, `does not (have|contain) (a )?(bug|deadlock)`,
`there is no (bug|deadlock|defect)`, `same order`, `no deadlock`.

Adjudication: A0 `claims_no_issue` accepts the **buggy input**
(`bug_present` by construction, `false_accept=True`). A1 `claims_no_issue`
accepts the most recent `build_ok=True` candidate; with none it is
`claims_no_issue_unbuilt` (not accepted). A2 is unchanged (tools green only).

## Single binary

All offline recomputation and this batch use `BIN_MAIN`
sha256 `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
(ConcIR `d3d59ed`, which includes the `holds_all` preservation hint). Any result
on another binary is listed as a deviation in RESULTS.

## RESULTS

`experiments/RESULTS.md` is **generated** by
`python -m cir_workflow results …` (see its header for the exact command and the
sha256 of every input). Do not edit RESULTS by hand; a SUMMARY/RESULTS mismatch
is a bug.

## A3_tiered escalation triggers (K-2 addendum)

- **T1 (original)**: escalate when the local phase does not accept within its
  rounds (stalled / repeated `explore_fail`).
- **T2 (early)**: escalate after the **first** local round when its diagnostic
  carries a `holds_all` / `never_holds_all` preservation hint and the candidate
  diff against the input only releases or reorders locks. Evidence:
  `case-partial-deadlock-v1/CASE.md` "Tiered escalation".
