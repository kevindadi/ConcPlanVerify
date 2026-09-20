# Paper evidence map

Each paper claim → the table/number that supports it → the command that
generates it → the raw artefact. Status: **ready / partial / missing**.

Binary: `BIN_MAIN 4bec943d…` (ConcIR `d3d59ed`) for the main batch; the offline
rebase after the `E102` fix is `03d343e5…` (0 verdict changes).

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| (a) | Verification-driven repair has 0 false-accepts, while the tool-driven baseline accepts buggy programs | A3_local 0/21, A3_whole 0/20, A3_tiered 0/21 false-accept; A0 11/24, A2-ml 3/24 (`cycle_3lock` 3/3 reps) | `python -m cir_workflow results …` | `experiments/RESULTS.md` §1, §Per-arm; `flash-repair-main-v1/run-…/SUMMARY.json` | **ready** |
| (a′) | Miri + Lockbud are green on the 3-lock cycle (the accepted `cycle_3lock` A0/A2 programs) | Track D `cycle_3lock` buggy: ConcIR FAIL, Miri clean 16, Lockbud clean; expert `bug_present=yes` | `scripts/rebuild_trackd.py`; `scripts/label_v2.py` | `experiments/detection-v3/TRACKD.json`; `expert-labels/EXPERT_LABELS.json` | **ready** |
| (b) | Local regeneration uses fewer tokens/rounds than whole-artifact revision at equal or better acceptance | A3_local 21/24, 683 tok/correct; A3_whole 20/24, 3627; A3_tiered 21/24, 684 | `results` | `RESULTS.md` §Per-arm | **ready** |
| (c) | The design-preservation contract blocks the empty fix | `partial_deadlock_bystander`: A3_local 0/3 (stalled) vs A3_whole 2/3; `holds_all` hint rejects release-early | `scripts/run_case_partial.py` | `case-partial-deadlock-v1/CASE.md` + run dir | **ready** |
| (d) | A3 products land in Rust and conform — with measured recall | conform recall: event-stream mutation M6 1.0 (`violation`); code-level mutations that preserve the event stream M1 0.545/M2 0.067/M4 0.333 (all `timeout`, i.e. behavior) and M3 0.0; M7 spawn-swap control 2/19 false-positive; post-edit drift-only-conform 0 (edits preserve the ev stream) | `scripts/mutate.py`; `scripts/post_edit.py`; `scripts/a3_to_rust.py` | `conform-mutation-v1/{SUMMARY.md,CONFORM_GAPS.md}`; `post-edit-conform-v1/SUMMARY.md`; `a3-to-rust-v1/SUMMARY.md` | **ready** |
| (e) | Local→whole escalation | main K=4: A3_tiered 21/24, 0 false-accept, 684 tok, escalation 3/24. `partial_deadlock` addendum: early escalation (1 local + 3 whole) 3/3; original trigger K=6 3/3 | `scripts/run_a3_tiered.py`; `scripts/run_tiered_partial.py` | `case-partial-deadlock-v1/CASE.md` "Tiered escalation"; `RESULTS.md` | **ready** |
| (f) | Detection capability: ConcIR decides every CIR case; dynamic/static Rust tools miss | Track D tables (10 Rust tasks + CIR-only appendix) | `scripts/rebuild_trackd.py`; `results` | `detection-v3/TRACKD.json`; `RESULTS.md` §Track D | **ready** |
| (g) | Scale | 24 generated lock-chain runs | `python -m cir_workflow scale` | `scale-v2/SCALE.json`; `RESULTS.md` §Scale | **ready** |
| (h) | Expert track + human review | per-candidate labels 54, unsure 4/54 = 7.4%, agreement 108/117 = 0.923, `design_loss` by arm | `scripts/label_v2.py` | `expert-labels/EXPERT_LABELS.{json,md}`, `HUMAN_REVIEW_QUEUE.md` | **partial** (human column blank; needs the owner) |
| (i) | Extraction track (third Rust oracle) | validated 0/63 → limitation | `scripts/run_extraction_v6.py` | `extraction-v6/EXTRACTION_LIMITS.md` | **ready** (as a limitation) |

| (j) | Generalization | main batch 10 tasks x 6 arms x 3 reps (added `abba_2lock`, `bare_wait_no_predicate`); A3_local 24/30, A3_whole 26/30 — whole wins on `bare_wait_no_predicate` (new predicate needed) while local is ~5x cheaper | `scripts/run_main_v1_extra.py`; `results` | `RESULTS.md` §1/§Per-arm | **partial** (no second model: endpoint aliases return Flash, `model-probe-v1/SUMMARY.md`) |

## Notes

- (a′) is the strongest single result: three repeats of a real 3-lock cycle pass
  Miri (16 seeds) and Lockbud, and are accepted by A2-ml, but the expert track
  marks `bug_present=yes` (sha `c7d11e85…`, all reps).
- `design_loss` (accepted ∧ design_preserved=no) is 6 cell-repeats; it is the
  contract-relevant complement of false-accept.
- The human review queue is intentionally blank; paper claims about human
  verification depend on the owner filling it.
