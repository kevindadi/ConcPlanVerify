# EVIDENCE_TIERS — experiment directory tiers (freeze-7)

Tier A: referenced by freeze-7 `RESULTS.md`, `tables/*.tex`, `FREEZE_MANIFEST.md` or
`docs/PAPER_EVIDENCE_MAP.md`. Tier B: version lineage needed for v1/v2 comparisons.
Tier C: superseded by a later version; archived under `experiments/_archive/` with a
`WHY_ARCHIVED.md`. Archiving uses `git mv`; freeze tags still reference the original
commits.

## Tier A

- `experiments/tables`
- `experiments/flash-gen-main-v4-code`
- `experiments/gen-code-replay-v2`
- `experiments/flash-repair-main-v1`
- `experiments/detection-v3`
- `experiments/scale-v2`
- `experiments/conform-mutation-v2`
- `experiments/post-edit-conform-v2`
- `experiments/extraction-v5`
- `experiments/model-probe-v2`
- `experiments/gen-model-probe-v2`
- `experiments/rust-oracle-v1`
- `experiments/gen-llmcode-smoke-v1`
- `experiments/case-partial-deadlock-v1`
- `experiments/a3-to-rust-v2`
- `experiments/contract-strength-v1`
- `experiments/extraction-v6`

## Tier B

- `experiments/deepseek-flash-repair-v1` (frozen inputs used by `build_families.py` and tests)

- `experiments/conform-mutation-v1`
- `experiments/post-edit-conform-v1`
- `experiments/flash-gen-main-v1`
- `experiments/flash-gen-main-v2`
- `experiments/flash-gen-main-v3-code`
- `experiments/gen-model-probe-v1`
- `experiments/gen-code-replay-v1`
- `experiments/conformance-v4`
- `experiments/scale-v1`
- `experiments/model-probe-v1`

## Tier C (archived)

- `experiments/_archive/conformance-v1`
- `experiments/_archive/conformance-v2`
- `experiments/_archive/conformance-v3`
- `experiments/_archive/conformance-v4-rebase`
- `experiments/_archive/deepseek-flash-pilot-v1`
- `experiments/_archive/detection-v1`
- `experiments/_archive/detection-v2`
- `experiments/_archive/extraction-v4`
- `experiments/_archive/flash-arms-smoke-v1`
- `experiments/_archive/flash-arms-v1`
- `experiments/_archive/flash-repair-smoke-v1`
- `experiments/_archive/flash-repair-smoke-v2`
- `experiments/_archive/flash-repair-smoke-v3`
- `experiments/_archive/offline-integration-v1`
- `experiments/_archive/pilot-v1`
- `experiments/_archive/pilot-v2`
- `experiments/_archive/pilot-v2-lifecycle`
- `experiments/_archive/rebase-03d343e5`
- `experiments/_archive/rebase-4bec943d`
- `experiments/_archive/a3-to-rust-v1`

## Duplicate files (counted, not removed; excluded when packaging)

- `cir_trace.rs` copies under `experiments/`: 2506
- `concir_sync.rs` copies under `experiments/`: 224
- empty `stderr.txt`: 3514

## Build output

`experiments/**/target/` (2466 dirs, ~12 GB) removed with `git clean -fdX -- experiments`;
that space is ignored build output, not repository data.
