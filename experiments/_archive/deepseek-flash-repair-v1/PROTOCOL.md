# PROTOCOL — deepseek-flash-repair-v1 (frozen before any model call)

Goal: a minimal, independently reviewable closed loop — **LLM proposes one
constrained CIR patch → Rust verifies the full frozen contract → real rejection
feedback → accept → independent replay** — using DeepSeek Flash on two frozen
defect models. Not an effect-size study; not a source-level repair claim.

## Model boundary (hard)

- Provider `deepseek`, model `deepseek-flash`, endpoint `https://api.deepseek.com`
  (Chat Completions). Pro/aliases/other providers/automatic fallback are
  forbidden; a non-allowed value fails before a request and a response `model`
  mismatch stops the batch.
- Thinking explicitly disabled: `extra_body={"thinking": {"type": "disabled"}}`.
- `max_tokens=4096`, per-request timeout 90 s, `temperature=0.0`, SDK
  `max_retries=0`, at most one transient transport retry (timeout/rate-limit/5xx).
- Key read from the application repository `.env` (`DEEPSEEK_API_KEY`); never
  printed, logged or written to any record. Missing SDK/import failures are fixed
  locally and never counted as HTTP attempts.

## Budget

| item | value |
| --- | --- |
| tasks | 2 (below), in order |
| rounds / task | 3 |
| batch HTTP cap | 6 actual attempts (incl. transport retries) |
| batch wall clock | 20 minutes |
| persistence | `budget.json` counted before each HTTP attempt; restart/new directory must not reset the same experiment |

Non-retryable errors (401/403/balance/invalid model/parameter) stop the batch
without switching models. Results are delivered even if incomplete.

## Frozen inputs

Copied byte-for-byte from `deepseek-flash-pilot-v1` (not regenerated); the run
validates these hashes before any patch:

| task | model sha256 | contract sha256 | root (reconfirmed) |
| --- | --- | --- | --- |
| `r1_t2_abba` (`t2_abba`) | `d2d4958ba7a5a7ac…` | `961458b818416972…` | FAIL complete |
| `r2_t3_cross_module_abba` (`t3_cross_module_abba`) | `a513fadc58079ab1…` | `a53d94331df2a250…` | FAIL complete |

`REPAIR_TASKS.json` holds the exact paths/hashes. Neither model nor contract is
modified; no accepted patch from the earlier pilot is given to the model.

## Patch protocol (Rust owns the rules)

- Context (`repair-context`): model/contract fingerprints, allowed scope, root
  verification + diagnostics, functions with lock `sid`s and Rust
  `function_hash`. No solved patch.
- Candidate (`concir-external-patch-candidate-v1`): `context_fingerprint`, and a
  single `swap_statements` between two adjacent `mutex_lock` statements with
  different resources, plus the target function's `original_hash`.
- Evaluation (`evaluate-patch`, `concir-external-patch-artifact-v1`): binds the
  context fingerprint, re-checks the target hash, the allowed scope, adjacency,
  distinct resources and non-control targets, then applies, statically
  validates, lowers/supports, re-binds the frozen contract and fully verifies it.
  Rejected: deletion, whole-program replacement, contract change, extra edits,
  out-of-scope or stale targets, static-invalid, UNKNOWN, UNSUPPORTED.
- Acceptance requires a **complete PASS of all properties and preserved
  behaviour**. `replay` re-applies and re-verifies the artifact offline from its
  embedded inputs and rejects tampering (including contradictory accept flags).
- Every candidate is evaluated against the same frozen initial model; rejection
  does not change the model; no multi-patch accumulation.

## Labels

`candidate_source` = `llm` (live) or `scripted` (offline); `validator` = `tool`;
`repair_mode` = `external_single_patch`. A deterministic tool repair is never
counted as an LLM success. The LLM only chooses the target and the swap; Rust
decides the verdict.

## Feedback loop

Per task: read frozen model/contract → Rust context → up to 3 rounds of
(propose candidate → evaluate) with the **real** structured rejection reason, the
original candidate and the frozen context fed back; on acceptance, Rust replay.
Parse/candidate rejections may use the budget; auth/model-identity/tool
process/protocol errors stop. Duplicate candidates are detected by content.

## Evidence

Sanitized exact system/user messages, prompt hash, raw assistant text, request id,
requested/response model, thinking setting, `finish_reason`, usage (or null),
timing, transport-retry index; and full Rust logs (argv, binary/input/contract/
artifact hashes, exit code, stdout/stderr, verification/replay results). Old v1
search artifacts remain replayable.
