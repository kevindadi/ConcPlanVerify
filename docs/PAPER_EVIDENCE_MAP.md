# Paper evidence map (v3 — generation-first)

Each claim → number → command → artefact → status. Ordered as the evaluation
will read: generation is primary, repair is the second study.

Binaries: `concir-backend` `cf4f9e9a…`, `concir-instrument` `5db7765c…`
(ConcIR `6e2a3de`, tag `concir-freeze-3`). Frozen at `experiments-v2-freeze-4`.

## (1) Generation: requirement satisfaction and defects (primary)

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| g1 | Model-first generation (G3) satisfies more requirements than direct/tool-loop Rust at equal-or-better tractability | G0 RF 0.422, G1 0.460, G2 0.468, **G3 0.504**; G3 RC 0.776; G3 20/72 accepted-with-proof | `scripts/render_gen_main.py …`; `python -m cir_workflow results … --gen …` | `flash-gen-main-v1/SUMMARY.md`; `tables/{gen_main,gen_arms,gen_tiers}.tex`; `RESULTS.md` §"Generation (main)" | **ready** |
| g2 | The advantage is not free: G3 costs ~10x tokens and is not defect-free | G3 539k tokens vs G0 58k; G3 defect 2 (model PASS, codegen hangs on `same_cv_different_locks`), G0 6, G1 1, G2 0 | same | `RESULTS.md` §"Generation (main)" | **ready** |
| g3 | Complexity tiers | 8 Simple / 8 Medium / 8 Complex; per-tier RF in `gen_tiers.tex` | `benchmarks/build_families.py --check-generation` | `benchmarks/GENERATION_MANIFEST.json`; `benchmarks/TIERS.md` | **ready** |

## (2) Frontier-model increment

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| p1 | With a strong base model, G3 still adds RF and stays defect-free | kimi-k3: G0 RF 0.499, G2 0.434, **G3 0.684**; G3 RC 0.776, defect 0, awp 10/24 | `scripts/run_gen_probe.py` | `gen-model-probe-v1/SUMMARY.md`; `tables/gen_probe.tex` | **ready** |

## (3) Landing and post-verification

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| l1 | Accepted CIRs land in buildable Rust and conform (operation-bound recall) | conform v2 recall M1 1.0, M2 0.947, M4 1.0, M6 1.0, M8 0.8; M5 0.0 blind spot; a3-to-rust-v2 23/23 conform PASS; post-edit drift 0 | `scripts/mutate_v2.py`; `scripts/post_edit_v2.py`; `scripts/a3_to_rust.py` | `conform-mutation-v2/{SUMMARY,CONFORM_GAPS}.md`; `post-edit-conform-v2/SUMMARY.md`; `tables/{mutation,conform_recall_v1v2,postedit}.tex` | **ready** |
| l2 | Free LLM Rust can be scored without a model | instrument v2 + monitor: 10/10 buggy references hang; 8/10 fixed references have every non-`[U]` requirement `PASS_bounded`; 2 explained instrument limits (`var_eq`, user semaphore) | `scripts/run_rust_oracle.py` | `rust-oracle-v1/{RESULTS.json,SUMMARY.md}`; `experiments/rust-oracle-v1/PROTOCOL.md` | **ready** |

## (4) Repair study (second study)

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| r1 | Verification-driven repair has 0 false-accepts; tool-driven baselines accept buggy programs | A3_local 0/21, A3_whole 0/20, A3_tiered 0/21; A0 11/24, A2-ml 3/24 (`cycle_3lock` 3/3) | `python -m cir_workflow results …` | `RESULTS.md` §1, §Per-arm; `flash-repair-main-v1/…/SUMMARY.json` | **ready** |
| r2 | Miri + Lockbud are green on the accepted 3-lock cycle | Track D `cycle_3lock` buggy: ConcIR FAIL, Miri clean 16, Lockbud clean, expert `bug_present=yes` | `scripts/rebuild_trackd.py` | `detection-v3/TRACKD.json`; `tables/trackd.tex` | **ready** |
| r3 | Local regeneration is cheaper than whole-artifact at equal acceptance | A3_local 24/30, 675 tok/correct; A3_whole 26/30, 3267; A3_tiered 24/30, 676 | `results` | `RESULTS.md` §Per-arm | **ready** |
| r4 | The design-preservation contract blocks the empty fix | `partial_deadlock_bystander`: A3_local 0/3 (stalled) vs A3_whole 2/3 | `scripts/run_case_partial.py` | `case-partial-deadlock-v1/CASE.md` | **ready** |

## (5) Detection capability

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| d1 | ConcIR decides every CIR case; dynamic/static Rust tools miss | Track D tables (10 Rust tasks + CIR-only appendix) | `scripts/rebuild_trackd.py`; `results` | `detection-v3/TRACKD.json`; `tables/trackd.tex` | **ready** |
| d2 | loom cross-check | not run | — | — | **missing** (§5 optional) |

## (6) Scale

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| s1 | State growth and the UNKNOWN boundary | 24 generated lock-chain runs | `python -m cir_workflow scale` | `scale-v2/SCALE.json`; `tables/scale.tex` | **ready** |

## (7) Expert track and human review

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| e1 | Agent-proxy labels agree with the automatic oracle; human review merged | per-candidate labels 54, unsure 4/54; agreement 108/117; human review 17 rows = 11 candidates, agent vs human 7/7 | `scripts/label_v2.py`; `scripts/merge_human_review.py` | `expert-labels/EXPERT_LABELS.json`; `tables/expert.tex`; `RESULTS.md` | **ready** |
| e2 | Generation-cell expert labels | not run | — | — | **missing** |

## (8) Extraction limitation

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| x1 | LLM extraction of CIR from Rust is a limitation | validated 0/63 | `scripts/run_extraction_v6.py` | `extraction-v6/EXTRACTION_LIMITS.md` | **ready (as limitation)** |

## (9) Threats to validity

- Single provider family for the main batch (DeepSeek Flash); the frontier probe
  covers one stronger model (kimi-k3, 1 rep).
- 24 tasks, 3 reps (main), 1 rep (probe).
- The Rust-arm oracle is **bounded** (`PASS_bounded`); the model arm is
  exhaustive. The two words are never mixed.
- G3 requires a **name-alignment** step (candidate resources/functions aligned
  to the contract by kind/declaration order) because the model cannot see the
  contract's names; recorded per cell.
- `var_eq`/`var_cmp` clauses are `unsupported` for free Rust (no value events);
  user-defined semaphores are `unmapped`.
- Expert labels are agent-proxy for the generation cells (not yet run).
- Miri is bounded; `deadlock_free` is resolved from native behavior.
