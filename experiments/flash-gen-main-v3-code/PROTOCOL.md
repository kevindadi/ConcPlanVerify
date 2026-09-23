# flash-gen-main-v3-code — protocol

Code-stage run. Sources and harness are pinned so each cell is traceable.

- **Input CIR**: `flash-gen-main-v2/run-20260923T005232` (the verified CIRs are not regenerated).
- **Harness (ConcIR)**: `concir-freeze-5` at `24fc982`; `concir_sync` path crate linked; one
  cargo project template shared by all arms.
- **Prompts (sha256)**: `rust_from_cir_v1.md` = 4d690d92…;
  `rust_generation_v1.md` = 41ac636f…; system + user text is recorded per cell
  under `llm/requests.jsonl`.
- **K_code** = 3 (G3 code stage); non-G3 arms K = 4.
- **Requests**: 112 (DeepSeek Flash).
- **Re-run**: all 59 G3 code cells.
- **Acceptance**: build ∧ `conform --op-resource` PASS ∧ monitor no FAIL ∧
  behavior; `accepted_with_proof` = model PASS ∧ conform PASS ∧ monitor no FAIL.
- Baselines not re-run here are carried from the source run and carry its
  `source_run` in `SUMMARY.json`.
