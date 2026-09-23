# flash-gen-main-v4-code — protocol

Code-stage run. Sources and harness are pinned so each cell is traceable.

- **Input CIR**: `flash-gen-main-v3-code/run-20260923T182939` (the verified CIRs are not regenerated).
- **Harness (ConcIR)**: `concir-freeze-6` at `fe34a43`; `concir_sync` path crate linked; one
  cargo project template shared by all arms.
- **Prompts (sha256)**: `rust_from_cir_v1.md` = 4d690d92…;
  `rust_generation_v1.md` = 41ac636f…; system + user text is recorded per cell
  under `llm/requests.jsonl`.
- **K_code** = 3 (G3 code stage); non-G3 arms K = 4.
- **Requests**: 140 (DeepSeek Flash).
- **Re-run**: all G3 + semaphore-family G0/G1/G2.
- **Acceptance**: build ∧ `conform --op-resource` PASS ∧ monitor no FAIL ∧
  behavior; `accepted_with_proof` = model PASS ∧ conform PASS ∧ monitor no FAIL.
- Baselines not re-run here are carried from the source run and carry its
  `source_run` in `SUMMARY.json`.
