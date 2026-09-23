# flash-repair-smoke-v3 — PROTOCOL (frozen 2026-09-18i)

Main multi-arm repair batch. 8 hard tasks (tool-blind-spot / LLM-prone types):
partial_deadlock_bystander, cross_module_cycle, cycle_3lock,
nested_scope_lock_order, notify_one_multi_waiter_wrong_pick,
bounded_backpressure_lock_held, send_while_holding_mutex,
acquire_twice_no_release. Each uses `repair_input/` (de-leaked, terminal `DONE`
line) and its frozen `contract.json`.

Arms: A0_direct, A1_self_iter, A2_tools_iter_ml (build+Miri+Lockbud), A3_local
(local regeneration), A3_whole (whole-artifact resend). K=4. Miri 16 seeds x 8 s
in the terminal oracle. Budget: repair arms <=200 requests, wall <=3 h; a shared
`budget.json`; stop and record `stop_reason` on exhaustion, keeping completed
cells.

Deviation D-19: this batch is **not gated** on Rust-arm oracle completeness;
`oracle.model` may be `inconclusive` here and is backfilled later by extraction /
expert labels, reported separately.
