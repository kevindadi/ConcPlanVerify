# REAL_CASES_V0_ERRATA

Errata for the `real-cases-v0` delivery in `/Users/kevin/local-repos/ConcIR`.
Original result files are unchanged except for the doc-text unit fix noted
below; this record corrects the reading, not the raw data.

## What the delivery is

- **One verifiable/reducible issue-derived model**: `rmw-zenoh-998` — a faithful
  two-`std::mutex` ABBA reduction of [ros2/rmw_zenoh #998](https://github.com/ros2/rmw_zenoh/issues/998).
  Root `FAIL` (49 states, complete); the backend's adjacent-swap repair finds a
  legal 1-swap witness under the frozen contract.
- **One support-boundary model**: `dashmap-369` — a faithful per-shard `RwLock`
  reduction of [xacrimon/dashmap #369](https://github.com/xacrimon/dashmap/issues/369).
  `UNSUPPORTED` in CIR v1; **not** rewritten as `Mutex` to force a success.

Neither case has a verified upstream buggy/fixed commit pair; `rmw-zenoh-998`'s
`fixed.cir.json` is a **hypothesis** (uniform lock order), not the upstream
patch (upstream suggests releasing `mutex_` before taking `condition_mutex`).
The reduction collapses the interval between the two lock acquisitions, which is
more adjacent-swap-friendly than the source; no source→CIR equivalence is claimed.

## Corrections

1. **Unit error.** `REAL_CASES_V0_HANDOFF.md` wrote "repaired v2 / 92 s". It
   means **2 verification calls, 92 cumulative states**, search times **8/8/9 ms**
   for A/B/C. The doc text is corrected; no timing/count value in the raw JSON
   changed. (`real-cases-v0` handoff, A/B/C table.)

2. **`attempted` must exclude `not_executed`.** `pilot_analyze.py` previously
   counted a B/C pair as "attempted" when both records merely existed, including
   a side that was never executed. It now distinguishes:
   - `both_present` (records exist) from `both_attempted` (neither side
     `not_executed`);
   - `repaired_vs_not_executed` is reported separately and is not a win/loss;
   - `success_differs` only counts pairs where both sides were attempted.
   Regression: `run_pilot.py lifecycle` check
   `pairing_keeps_timeout_and_not_executed` (planned 4 / attempted 3 /
   both_repaired 1 / success differences 2 / not_executed 1 /
   repaired_vs_not_executed 1 / cost subset 1).

3. **Cumulative cost counts executed stages only.** `_cumulative_attempt_ms`
   summed `wall_ms` (now the search cost), so a replay-only recovery double
   counted the inherited search and omitted replay. It now adds
   `search_wall_ms` for attempts that actually searched plus `replay_wall_ms`,
   and for a replay-only recovery adds **only** `replay_wall_ms`. A 100 ms search
   followed by a 20 ms recovery replay totals 120 ms, not 200 ms. Strategy
   comparison still uses `search_wall_ms`.

4. **Generator identity is explicit.** The runner hard-coded
   `generate_cases.py`; the real generator is `build_cases.py`. The manifest can
   now name the generator (real-cases-v0 sets `"generator": "build_cases.py"`),
   and the cohort records its path identity and sha256. New batches
   `rc-root2`/`rc-abc2` record
   `generator_sha256=264dc43dd86ebbbc…`, `generator_path_identity=
   experiments/real-cases-v0/build_cases.py`. Old batches are **not** backfilled;
   their generator identity stays as recorded.

## Evidence for the corrections

- New batches under `experiments/real-cases-v0/results/batches/`:
  `rc-root2` (3/3 complete) and `rc-abc2` (6/6 complete).
- `python3 scripts/pilot_audit.py experiments/real-cases-v0` → 0 issues across
  `rc-root`, `rc-root2`, `rc-abc`, `rc-abc2`.
- `scripts/run_pilot.py lifecycle` → 11/11 (includes the `not_executed` pairing
  and comparable-search-cost checks); `scripts/pilot_checks.py` → 3/3.
- Core unchanged: `INSTA_UPDATE=no cargo test --offline --all-targets
  --no-fail-fast` → 229 passed / 0 failed / 0 warnings.
