# scripts

Active scripts and the experiment directory / freeze they produce. Superseded
one-off scripts live in `scripts/legacy/`.

| script | produces | freeze |
| --- | --- | --- |
| `run_gen_main.py` | `flash-gen-main-v2` (4 arms + `G3_codegen`) | freeze-6 |
| `run_gen_v3_code.py` | `flash-gen-main-v3-code` (code stage only) | freeze-6 |
| `run_gen_v4_code.py` | `flash-gen-main-v4-code` (code stage + semaphore baseline) | freeze-7 |
| `render_gen_main.py` | batch `SUMMARY.md` + `tables/gen_*.tex` | freeze-7 |
| `run_gen_probe.py`, `run_gen_probe_v2.py` | `gen-model-probe-v1` / `v2` | freeze-5/6 |
| `run_llmcode_smoke.py` | `gen-llmcode-smoke-v1` | freeze-6 |
| `replay_gen_code.py`, `replay_gen_code_v2.py` | `gen-code-replay-v1` / `v2` | freeze-6/7 |
| `render_replay.py` | `tables/gen_replay.tex` | freeze-7 |
| `run_rust_oracle.py` | `rust-oracle-v1` (bounded monitor acceptance) | freeze-5 |
| `rebuild_trackd.py` | `detection-v3/TRACKD.json` | freeze-7 |
| `label_v2.py`, `label_main_v1.py`, `merge_human_review.py`, `human_disagreement_evidence.py` | expert labels + human queue/merge | freeze-7 |
| `mutate_v2.py` | `conform-mutation-v2` | freeze-7 |
| `post_edit_v2.py`, `post_edit_v2_forced.py` | `post-edit-conform-v2` | freeze-7 |
| `a3_to_rust.py` | `a3-to-rust-v2` | freeze-7 |
| `run_extraction_v6.py` | `extraction-v6` | freeze-7 |
| `model_probe_v2.py`, `model_probe_v2_complete.py` | `model-probe-v2` | freeze-7 |
| `run_a3_tiered.py`, `run_tiered_partial.py`, `run_case_partial.py`, `run_main_v1_extra.py` | repair study case/batch additions | freeze-7 |

Regeneration of `RESULTS.md` and `tables/*.tex` is done by
`python -m cir_workflow results …` (see `experiments/tables/*.tex` headers).
