# Paper evidence map (v4 — requirements → verified CIR → LLM code)

Ordered as the evaluation reads. Binaries: `concir-backend` (freeze-5),
`concir-instrument`. Frozen at `experiments-v2-freeze-5` / tag `contracts-v3.1`.

## (1) Requirement satisfaction and defects (primary)

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| g1 | Model-first generation satisfies the most requirement clauses (RF_all), and the tool-loop arm is the only zero-defect one | G0 RF_all 0.523 / RF_acc 0.531, G1 0.549/0.541, G2 0.592/0.661 (defect 0), G3_concir 0.670/0.670 (defect 0); G0 defect 6, G1 3 | `scripts/render_gen_main.py`; `results --gen` | `flash-gen-main-v2/SUMMARY.md`; `tables/{gen_main,gen_arms,gen_tiers}.tex`; `RESULTS.md` §"Generation (main)" | **ready** |
| g2 | Complexity tiers | 8 Simple / 8 Medium / 8 Complex, 24 tasks, entities in requirements (v3.1) | `benchmarks/build_families.py --check-generation` | `benchmarks/GENERATION_MANIFEST.json` v3.1; `benchmarks/TIERS.md` | **ready** |

## (2) From verified CIR to code: LLM vs tool codegen

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| c1 | Entity names in the requirements lift the CIR stage (INVALID collapses) | G3 CIR stage **59/72 PASS** (v1 had 25/72 accepted, 46 INVALID); only `name_alignment` removed | `scripts/run_gen_main.py` | `flash-gen-main-v2` G3 cells | **ready** |
| c2 | The LLM-code stage is the current bottleneck; tool codegen still lands more | G3_concir accept 31/72 (rep0 10/24), 0.431 overall; G3_codegen ablation accept 20/24, RF_acc 0.765 vs 0.670 | `results --gen` | `tables/gen_codegen_ablation.tex`; `tables/gen_g3_stages.tex` | **ready** |
| c3 | Free-Rust smoke: code fidelity is the limit, not instrument coverage | 45 CIRs, build 45/45, **conform PASS 15/45 (33%)**; instrument limits empty | `scripts/run_llmcode_smoke.py` | `gen-llmcode-smoke-v1/SUMMARY.md` | **ready** |

## (3) Conform catches deviations the model verdict cannot

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| v1 | Post-verification rejects code rounds | **30 of 59** G3 code cells had `conform` reject at least one round | `results --gen` | `tables/gen_conform_value.tex` | **ready** |
| v2 | Operation-bound conform recall | M1 1.0, M2 0.947, M4 1.0, M6 1.0, M8 0.8; M5 0.0; M7 order-sensitive | `scripts/mutate_v2.py` | `conform-mutation-v2/{SUMMARY,CONFORM_GAPS}.md`; `tables/{mutation,conform_recall_v1v2}.tex` | **ready** |

## (4) Frontier model

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| f1 | kimi-k3 G3 v2 still leads G0 on RF_all | G3 0.598 vs G0 0.509; G3 accept 0.458, awp 11/24 | `scripts/run_gen_probe_v2.py` | `gen-model-probe-v2/SUMMARY.md`; `tables/gen_probe.tex` | **ready** |

## (5) Cost

| # | claim | number | command | artefact | status |
| --- | --- | --- | --- | --- | --- |
| k1 | G3 splits into CIR and code stages; code stage dominates rounds | CIR mean 1.54 rounds / 346k tokens; code mean 2.0 rounds / 221k tokens | `results --gen` | `tables/gen_g3_stages.tex` | **ready** |

## (6) Repair study (second study)

| # | claim | number | reference | status |
| --- | --- | --- | --- | --- |
| r1 | Repair has 0 false-accepts; tool-driven baseline accepts a green 3-lock cycle | A3_local 0/21, A3_whole 0/20; A0 11/24, A2-ml 3/24 (`cycle_3lock`) | `tables/main.tex`, `tables/arms.tex`, `tables/trackd.tex` | **ready** |

## (7) Detection capability and scale

| # | claim | number | reference | status |
| --- | --- | --- | --- | --- |
| d1 | ConcIR decides every CIR case; Miri/Lockbud miss the cycle | Track D | `tables/trackd.tex` | **ready** |
| d2 | State growth and the UNKNOWN boundary | 24 lock-chain runs | `tables/scale.tex` | **ready** |
| d3 | loom cross-check | not run | — | **missing** |

## (8) Expert track and human review

| # | claim | number | reference | status |
| --- | --- | --- | --- | --- |
| e1 | Agent-proxy labels; human review merged (repair study) | agreement 108/117; human 11 candidates | `tables/expert.tex` | **ready** |
| e2 | Generation-cell expert labels | not run | — | **missing** |

## (9) Extraction limitation

| # | claim | number | reference | status |
| --- | --- | --- | --- | --- |
| x1 | CIR extraction from Rust failed | 0/63 | `extraction-v6/EXTRACTION_LIMITS.md` | **ready (limitation)** |

## (10) Threats

- Author-constructed requirements/contracts; entity names are part of the requirements.
- G3's code stage is bounded by the LLM's fidelity to the CIR (33% smoke).
- Main batch single provider (DeepSeek Flash); probe one model, one rep.
- Rust-arm oracle bounded (`PASS_bounded`); model arm exhaustive; words not mixed.
- `same_cv_different_locks` is `UNSUPPORTED` (ConcIR W1xx); `var_eq` and user semaphores are instrument limits.
- Expert labels for generation cells not run; loom not run.

## v5 update (freeze-6): harness fixes, corrected RF, live code stage

- O-2 corrected metrics (RF_all counts non-accepted as 0):
  G0 0.472/0.531, G1 0.390/0.541, G2 0.376/0.661,
  **G3_concir 0.344/0.619** (accept 0.556, awp 40/72, defect 0),
  G3_codegen ablation 0.638/0.765.
- Post-verification value now separates harness gaps from real deviations:
  offline replay of the 28 freeze-5 code failures fixes only 2 on the **old**
  programs (they predate `concir_sync`); the live code stage with the fixed
  tools/prompt raises accept 0.431→0.556 and awp 31→40. Remaining failures are
  `extra_op` real deviations.
- Tables: `tables/gen_replay.tex` (harness-gap diagnostic); `gen_main.tex` with
  RF_all/RF_acc.
- Claims to rest on: G3 `RF_acc`/awp/Complex tier/`accepted_with_proof`, the
  ablation comparison, and conform's independent catches — **not** on RF_all
  where G3 < G0.

## v6 update (freeze-7): concir_sync crate, post-join main, v4 code

- Harness: `concir_sync` is a path-dependency crate (no duplicate `mod`), conform
  allows post-join main ops, `check` warns `W201` on declared-unused resources.
- Replay v2 (`gen-code-replay-v2`): 19 freeze-6 failures → 12 build-fixed + 5
  conform-fixed, 2 real (`extra_op`); 0 regressions.
- Live `flash-gen-main-v4-code`: G3 accept **0.750**, `accepted_with_proof`
  **54/72**, defect 0; G3 leads RF_all (0.493 vs G0 0.401), RF_acc 0.657 (above
  G1, below G2 0.683 and the codegen ablation 0.765).
- Limitation: channel `Mutex<Receiver>` wrappers still yield `unmapped`
  (37/40 accepted cells); prompt-only mitigation.
