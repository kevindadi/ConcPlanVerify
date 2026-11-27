# ConcPlanVerify v5 Examples (Stage 0-Driven)

Each example under `examples/<name>/` starts from a short, macro-level natural-language user specification (`spec.md`) without naming threads, locks, or low-level variable bounds. Running Stage 0 + Stage 1–4 with local DeepSeek (`deepseek-chat`) produces and verifies:

- `spec.md` — Macro-level natural-language requirement input
- `REQUIREMENTS.md` & `requirements.json` — Stage 0.1 typed symbol anchoring (`roles`, typed `resource_specs`, `domain_types`) + classified `R1..Rn` (`[V]` / `[U]`)
- `contract.json` & `contract_explanation.json` — Stage 0.2 isolated CTL/LTL + classic contract with `non_vacuous: true` implications and deterministic NL back-translation (`Stage 0.3`)
- `model.scir.json` — Stage 1/2 SCIR v5.0 concurrency model passing `lower` -> `check` -> `support` -> `summarize` -> `explore --engine petri`
- `traceability_manifest.json` & `verification_summary.json` — End-to-end verification report linking clauses -> properties -> SCIR nodes -> Rust holes -> `spn` MIR refinement (`e2e_status: "verified"`)

## Catalog

| Example | Macro Input (`spec.md`) | Sync Primitives & Domain Types | Verified Status |
| --- | --- | --- | --- |
| [`account_transfer`](account_transfer/spec.md) | Bidirectional concurrent bank transfers with receipt generation | `Mutex` (`account_a_lock`, `account_b_lock`), `Var` (`total_balance`, `receipt_counter`), `TypeHole` (`Receipt`) | Stage 0 + UniPN CTL + `spn` MIR Pass |
| [`config_cache`](config_cache/spec.md) | Concurrent read/write in-memory configuration cache | `RwLock` (`cache_lock`), `Var` (`pending_changes`), `TypeHole` (`ConfigSnapshot`) | Stage 0 + UniPN CTL + `spn` MIR Pass |
| [`task_handoff`](task_handoff/spec.md) | Upstream/downstream task handoff with checksum computation | `Channel` (`task_channel`), `Mutex` (`state_lock`), `Var` (`pending_tasks`, `processed_tasks`), `TypeHole` (`TaskPayload`, `ChecksumResult`) | Stage 0 + UniPN CTL + `spn` MIR Pass |
| [`visit_counter`](visit_counter/spec.md) | Multi-request concurrent visit statistics and summary payload | `Mutex` (`stats_lock`), `Var` (`visit_count`, `summary_ready`), `TypeHole` (`VisitSummary`) | Stage 0 + UniPN CTL + `spn` MIR Pass |

## Running Examples via CLI

```bash
# List all examples
PYTHONPATH=src python -m concplan list-examples

# Stage 0 Entry A: formalize directly from natural-language spec.md (requires DEEPSEEK_APIKEY / DEEPSEEK_API_KEY in .env)
PYTHONPATH=src python -m concplan --out out/stage0 formalize \
  --example visit_counter --from-nl --provider deepseek --model deepseek-chat

# Stage 0 Entry B: validate & back-translate pre-formalized requirements + contract (offline)
PYTHONPATH=src python -m concplan --out out/stage0-b formalize --example visit_counter

# Full Stage 0 -> Stage 4 E2E from macro natural-language spec.md
PYTHONPATH=src python -m concplan --out out/e2e e2e \
  --example config_cache --from-nl --provider deepseek --model deepseek-chat \
  --run-summarize --report out/e2e_report.json
```
