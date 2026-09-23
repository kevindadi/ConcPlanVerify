# CLI_CONTRACT — current `concir-backend` as consumed by the Python client

Source of truth: `/Users/kevin/local-repos/ConcIR/src/bin/concir-backend.rs` and
`doc/backend-usage.md`. The Python client (`python/cir_workflow/concir_client.py`)
wraps the commands below. It never re-implements the semantics.

## Commands and exit codes

| command | invocation | exit 0 | exit 1 | exit 2 | exit 3 | exit 4 | exit 5 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `check` | `check <program.json>` | valid | — | usage | — | invalid | — |
| `support` | `support <program.json>` | supported | — | usage | — | invalid/lower error | unsupported |
| `explore` | `explore <program.json> <contract.json> [petri\|interp]` | PASS | FAIL | usage | UNKNOWN | INVALID | UNSUPPORTED |
| `repair` | `repair <program.json> <contract.json> --strategy a\|b\|c [--candidate-budget N] [--verification-budget N] [--max-depth N] [--max-total-edits N] --artifact out.json` | repaired / already_satisfied | no_acceptable_candidate / budget_exhausted | usage | analysis_unknown | invalid / invalid_config | unsupported |
| `replay` | `replay <artifact.json>` | replayed | — | usage | — | replay failed | — |

`exit 0` is **not** a verdict shortcut:

- `check` success means the program parses and statically validates, not that a
  property holds.
- `support` success means the constructs are supported, not that a property holds.
- `explore` `PASS` is a complete pass; `FAIL` may be complete or incomplete;
  `UNKNOWN` means the bounded analysis did not finish and is not "safe".
- `repair` `repaired`/`already_satisfied` are success; the other outcomes are
  legitimate non-success results, not crashes.

## Payload fields used by the client

- `check` → `{valid: bool, diagnostics: [{severity, code, message, location}]}`.
- `support` → `{supported: bool, unsupported: [{construct, detail, location}]}`.
- `explore` (VerificationReport) → `outcome`, `complete`, `states_explored`,
  `transitions_explored`, `analysis_started`, `bounds`, `assumptions`,
  `model_fingerprint`, `contract_fingerprint`, `properties: [{id, outcome,
  detail}]`, `diagnostics: [{property, outcome, message, complete,
  counterexample, blocked, cir_statements, proven_facts, repair_hints}]`,
  `boundary_events`, `unsupported`, `invalid`.
- `repair` (SearchArtifact, schema `concir-repair-artifact-v1`) → `source`,
  `input_program`, `frozen_contract`, `effective_config` (strategy +
  budgets/depth/edits + bounds), `nodes`, `attempts`, `patch_chain`,
  `accepted_node`, `accepted_program`, `accepted_report`, `outcome`,
  `stop_reason`, `saw_unknown`, `truncation`, `counts`, `reproduce`.
- `replay` (ReplayResult) → `nodes`, `input_outcome`, `accepted_ok`,
  `accepted_node`, `chain_len`, `outcome`. On failure the tool writes nothing to
  stdout and exits 4 with `artifact replay failed: <reason>` on stderr.

## Client-side failure classes

The client separates the backend's semantic outcome from process/protocol
failures:

| `kind` | meaning |
| --- | --- |
| `semantic` | the backend ran and produced its documented outcome (including exit 1/3/4/5) |
| `usage_error` | exit 2 (argument/input error) |
| `process_error` | spawn failure, timeout, or missing input file |
| `protocol_error` | empty/non-JSON stdout, non-object payload, or a payload whose outcome contradicts the exit code |

## Artifact binding

A `repair` artifact is treated as this invocation's verified result only after:

1. `effective_config.strategy` and budgets/depth/edits equal the requested config;
2. the artifact's root report `model_fingerprint`/`contract_fingerprint` equal the
   fingerprints of the `explore` run for the same frozen CIR and contract;
3. `replay` succeeds.

This binding reuses the backend's own fingerprints; Python does not compute CIR
semantics. `replay` re-validates the artifact's internal consistency and chain.

## Known gaps

- **Legacy positional patch repair.** `repair <model> <contract> <patches.json>
  [budget]` prints a legacy patch report, not a replayable search artifact. It is
  deliberately not wrapped; treating it as an artifact would be wrong.
- **External LLM patch protocol.** There is no supported way for provider-proposed
  edits to enter the composite strategy search and produce a replayable artifact.
  This round closes the strategy-mode loop only (backend-generated repair).
- **RwLock.** `explore`/`repair` return `UNSUPPORTED` for RwLock programs; there
  is no RwLock candidate operation.
- **Input normalization.** `concir-backend` exposes no standalone "normalize"
  command; external file binding relies on the `explore` fingerprints above, not
  on a byte/AST normalization CLI.
