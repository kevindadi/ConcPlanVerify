# Flash smoke batch — PROTOCOL (frozen 2026-09-18b)

This smoke batch validates the multi-arm chain on a few frozen tasks. It is not
an effect-size study. It inherits `experiments/EXPERIMENTS_V2_PROTOCOL.md` and
registers the deviations below.

## Scope

- Directory: `experiments/flash-arms-smoke-v1/`.
- Tasks (3, chosen to cover three feedback types):
  - `lock-order/abba_2lock` — deadlock, single-patch fixable;
  - `condvar/lost_wakeup_notify_before_wait` — deadlock, **not** swap-fixable,
    exercises whole-CIR revision (A3);
  - `lock-order/partial_deadlock_bystander` — `always_reachable` goal-layer
    failure, exercises goal feedback.
- Arms: `A0_direct`, `A1_self_iter`, `A2_tools_iter`, `A3_ours_revision`,
  `A3p_ours_patch` (only where a buggy CIR exists), `A3_tool_repair`.
  Ablation arms are not in the smoke batch.

## Frozen constants

| constant | value |
| --- | --- |
| model | `deepseek-flash` (provider `deepseek`) |
| thinking / temperature / max_tokens / timeout | disabled / 0 / 4096 / 90 s |
| SDK retries | 0, plus ≤ 1 transient retry |
| K | 4 |
| HTTP cap | 48 |
| wall cap | 3600 s |
| budget file | `<batch>/budget.json` (shared, restart never resets) |
| key | `.env` only, never written to any artifact |

## Confirmation

`arms`/`flash_smoke.assert_protocol_confirmed` refuses to run unless the sha256
of this file equals the `--protocol-sha256` credential. The credential is the
sha256 of the **committed** protocol content; any edit changes it and re-freezes
the batch.

## Known deviations / limits

- D-1: Miri uses one many-seeds pass (0..64) plus the 5 frozen single-seed
  combinations; a miss is not safety.
- D-2: the task set is the capability families, not paper Table 1.
- D-3: Rust behavior tests are reported `null` when not authored; never `true`
  for a zero-test project.
- `A3p_ours_patch` is `not_applicable` when the case has no buggy CIR or the
  defect is not a lock-order swap.
- The ConcIR oracle for Rust artifacts reports `bug_present = null` until a
  per-case rule is authored; it never infers safety from a detector miss.

## Outputs

`SUMMARY.json` / `SUMMARY.md` under the batch directory: per arm × task
acceptance, terminal-oracle evidence, rounds, tokens, LLM wall-clock and tool
wall-clock. First-round acceptances are reported as "feedback not triggered".
