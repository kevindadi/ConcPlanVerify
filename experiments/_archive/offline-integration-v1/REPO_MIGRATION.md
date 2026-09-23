# REPO_MIGRATION — ConcPlanVerify old `cir2cvn` → current `concir-backend`

Date: 2026-09-18. Repository: `/Users/kevin/local-repos/ConcPlanVerify`
(HEAD at migration start: `080627347582a7800e94343062da7a855a211771`).
Cleanup was explicitly authorized by the user. Nothing was committed; deletions
are working-tree changes. No credentials were read, printed or removed; `.env`
and `.git` were untouched.

## Why

The old application was a `cir2cvn` crate that (a) re-implemented ConcIR→CVN
translation, verification and a repair library inside ConcPlanVerify and (b)
drove it through `--validate/--analyze/--goals` with a flat
`resources`/`functions`/`op`/`transfer` schema. The current architecture puts all
formal work in ConcIR's `concir-backend` CLI and keeps Python for generation,
prompts and orchestration. The old crate is a duplicate backend serving a retired
protocol.

## Deleted (working tree)

| path | reason |
| --- | --- |
| `Cargo.toml`, `Cargo.lock` | crate manifest for the retired `cir2cvn` translator; the app is Python now |
| `src/` (`lib.rs`, `main.rs`, `error.rs`, `verification.rs`, `goals.rs`, `validate.rs`, `translator/`, `repair/`) | duplicate ConcIR→CVN translator/verifier/repair implementation |
| `tests/` (Rust: `category1_*`, `category2_*`, `verification.rs`, `cir_schema.rs`, `generate_dots.rs`, `common/`) | tests for the retired translator |
| `python/cir_workflow/rust_cli.py` | old stdin `cir2cvn --validate/--analyze/--goals` client and `verified_safe` assumption |
| `python/cir_workflow/generation.py`, `repair.py` | workflows built on the old protocol and old prompts |
| `python/cir_workflow/prompts.py`, `prompt_assets/*` (old) | flat-schema prompts |
| `python/cir_workflow/plan.py`, `merge.py`, `repair_local.py`, `external.py`, `codegen.py`, `scaling.py`, `experiment.py`, `metrics.py` | only served the old crate/protocol/benchmarks |
| `python/tests/test_rust_cli.py`, `test_workflows.py`, `test_prompts.py`, `test_plan.py`, `test_merge.py`, `test_repair_local.py` | tests of the retired modules |
| `benchmarks/`, `doc/`, `skills/`, `canvases/` | old-schema benchmark data, translator docs, `cir2cvn` skill scripts, and the old Vite experiment report |
| `target/`, `canvases/node_modules/` | build artifacts (ignored) |

## Preserved (migrated to the paper repository)

Copied to `/Users/kevin/paper-review/papers/ConcPlanVerify/experiments/offline-integration-v1/legacy-cir2cvn/`
(133 tracked files, 848 KB):

- `benchmarks/` (old-schema CIR benchmark data) and `tests/e2e/`, `tests/fixtures/`.
- `doc/` (translator architecture/CVN/error codes) and `skills/concir-verify/`.
- `canvases/` tracked sources (old experiment report UI).
- the retired Python modules and prompt assets, plus the old `README.md`.

These are history/source evidence; they are not part of the current app and were
not deleted, only moved.

## Kept in the application repository

| path | role |
| --- | --- |
| `python/cir_workflow/concir_client.py` | **new** typed client for `concir-backend` |
| `python/cir_workflow/providers.py` | **new** provider protocol, `ScriptedProvider`, LLM adapter |
| `python/cir_workflow/offline_workflow.py` | **new** generation→check→feedback→verify→tool-repair loop |
| `python/cir_workflow/prompts.py` + `prompt_assets/concir_*_v1.md` | **new** modular-CIR prompts and structured feedback |
| `python/cir_workflow/llm.py` | real DeepSeek/Qwen SDK adapters (library only) |
| `python/cir_workflow/models.py` | slimmed to `ModelConfig` + token-usage helpers |
| `python/cir_workflow/env.py`, `json_utils.py` | unchanged helpers |
| `python/tests/test_concir_client.py`, `test_concir_integration.py`, `test_offline_workflow.py` | **new** coverage (real CLI for the latter two) |
| `python/tests/test_llm.py`, `test_env.py`, `test_json_utils.py` | retained provider/helper tests |
| `python/tests/fixtures/programs/` | small CIR/contract fixtures |
| `README.md` | rewritten to describe only current functionality |

## Replaced

- `RustCli` (stdin `cir2cvn`) → `ConcirClient` (file-path `concir-backend`,
  per-call directory, typed exit/JSON mapping, process-group timeout).
- flat-schema prompts → modular-CIR prompts (`concir_generation_v1.md`,
  `concir_feedback_v1.md`), with sha256 recorded in the experiment manifest.
- LLM `generate`/`repair`/`plan`/`merge` CLI commands → `offline` (scripted) plus
  the `CandidateProvider` interface; real LLM providers remain library-level and
  are not wired to a network command this round.

## Retained but unreferenced

- `unipn/` — a local nested git repository at `unipn/` (commit `7a403b39`). It
  was untracked before the migration; the cleanup commit recorded it in the
  index as a gitlink (mode `160000`) with **no `.gitmodules` entry**, and the
  current app does not reference it (the old crate's UniPN dependency went away
  with `Cargo.toml`). The directory was kept per the instruction not to delete
  by name/untracked status. Recommended next step: either remove the stale
  gitlink from the index or add a proper submodule entry — decide explicitly.
- `python/.venv` — local virtualenv (gitignored). The offline tests do not need
  it; `openai` is only required for real LLM providers.

## Commits

The migration was recorded by the repository owner in two commits (author
`zhangkw`): `31f905e` (remove the retired crate/benchmarks) and `c8322ae`
(rewrite README/CLI to the new workflow). The `unipn` gitlink above was
introduced by that cleanup and is flagged for an explicit decision.
