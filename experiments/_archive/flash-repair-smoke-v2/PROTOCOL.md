# Flash repair smoke v2 — PROTOCOL (frozen 2026-09-18f)

Repair-type smoke on de-leaked inputs, with the three-column Rust oracle.

- Tasks: `lock-order/abba_2lock`, `lock-order/partial_deadlock_bystander`,
  `condvar/bare_wait_no_predicate`, all read from `repair_input/`
  (`repair_task.json`).
- Arms: `A0_direct`, `A1_self_iter`, `A2_tools_iter_m`, `A2_tools_iter_ml`,
  `A3_ours_revision` (v2 prompt + normalize + stall/local-patch).
- K=4. Budget: repair <=48 requests, extraction <=24, total <=72. Shared
  `budget.json`; restart never resets.
- Oracle columns (never fused): `oracle.build`, `oracle.behavior` (10 s watchdog
  run of the built candidate; timeout = hang), `oracle.miri` (16 seeds, 8 s each,
  `thread_leak`/`timeout`/`detected`/`clean`), `oracle.model` (LLM extraction +
  conformance validation; `extract_unverified` when not validated).
- `bug_present = behavior hang or model_verdict FAIL(validated)`;
  `false_accept = accepted and bug_present`.
- Model rules: `deepseek-flash`, thinking disabled, temperature 0,
  `max_tokens=4096`, timeout 90 s, SDK retries 0 + <=1 transient.
