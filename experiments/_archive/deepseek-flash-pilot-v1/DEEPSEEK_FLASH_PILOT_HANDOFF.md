# DEEPSEEK_FLASH_PILOT_HANDOFF

First small, controlled real-model pilot of the ConcIR generation loop using
**DeepSeek Flash**. This is a link/generation feasibility check — not an FSE-scale
evaluation, not a claim that the LLM performs constrained patch repair, and not a
general-effectiveness claim. The batch stopped after its three tasks; it was not
expanded and the model was not switched.

Repositories: Python app `/Users/kevin/local-repos/ConcPlanVerify` (this work),
Rust CLI `/Users/kevin/local-repos/ConcIR` (unchanged core), evidence
`experiments/deepseek-flash-pilot-v1/` (this directory).

## 1. Acceptance fixes (before any model call)

Two reviewed acceptance holes were fixed and regression-covered:

1. **Repaired without an artifact is no longer a success.**
   `offline_workflow.OfflineWorkflow.run` now requires, for a `repaired`/
   `already_satisfied` repair outcome, that this run's artifact exists, binds to
   the invocation (`effective_config` + `explore` `model_fingerprint`/
   `contract_fingerprint`), and passes `replay`; otherwise the run is
   `tool_error` and `repaired_by_tool` is `False`. `repaired_by_tool` also
   requires a recorded successful replay.
   Regression: `test_offline_workflow.py::...::test_repaired_without_artifact_is_tool_error`.

2. **Replay success requires a real ReplayResult.**
   `concir_client._validate_replay_payload` checks exit 0 *and* a structured
   ReplayResult (`nodes`, `input_outcome`, `accepted_ok`, `accepted_node`,
   `chain_len`, `outcome`) with types, and its correspondence to the artifact
   (`chain_len` vs `patch_chain`, `input_outcome` vs root report outcome,
   `accepted_node`). Empty/garbage/missing/contradictory output is a
   `protocol_error`, never `replayed`.
   Regression: `test_concir_client.py` replay tests, and
   `test_offline_workflow.py::...::test_bad_replay_result_is_tool_error`.

Also added: each workflow run gets an exclusive output directory
(`_exclusive_run_dir`), so repeated commands cannot overwrite an earlier run's
generation/contract/report (`test_each_run_has_exclusive_output_dir`).

The full offline suite (scripted, **no key, no network**) is now **44 tests / OK**;
`test-results.txt`.

## 2. Controlled live entry

`python/cir_workflow/live.py` + the CLI `live` command:

- provider fixed to `deepseek`, model fixed to `deepseek-flash`, endpoint
  `https://api.deepseek.com` (Chat Completions). A non-allowed value fails
  before any request (CLI allow-list + `assert_allowed_model`); the response
  `model` is compared to the request and a mismatch raises `ModelIdentityError`
  and stops the batch. No Pro/alias/fallback.
- thinking explicitly disabled: `extra_body={"thinking": {"type": "disabled"}}`
  (confirmed against the DeepSeek thinking-mode docs; omitting the field does
  **not** disable thinking).
- `max_tokens=4096`, per-request timeout 90 s, `temperature=0.0`, SDK
  `max_retries=0` so transport retries are only ours (at most 1, only for
  timeout/rate-limit/5xx).
- a single persisted budget (`budget.json`) counted before each HTTP attempt and
  a batch wall-clock deadline; failed attempts and restarts consume the budget.
- every attempt recorded sanitized (exact messages, prompt sha256, requested and
  response model, request id, `finish_reason`, usage or `null`, timing, retry
  index) with no key or request headers.

`live.py` also contains the small, structural modelling checks
(`structural.py`) used to judge fidelity independently of the tool verdict.

## 3. Batch and budget actually used

- Batch: `runs/run-20260918T090459-72384-02cd06`.
- Real HTTP attempts: **3 / 12** (one per task, first candidate passed static
  checking each time); 9 remaining; batch stop reason: none; wall time ≈ 8 s of
  model calls. No transport retry occurred.
- An earlier setup attempt (`runs/run-20260918T090449-72325-81307e`) failed with
  `No module named 'openai'` on the system Python and made **zero** HTTP
  requests; it is kept as evidence and is not part of the real batch. The run
  used the repository virtualenv (`python/.venv`, `openai 2.0.0`).

## 4. Results (three frozen tasks)

Script-generated table: `SUMMARY.md` / `SUMMARY.json`. Contract and prompts were
frozen before the run (hashes in `PROTOCOL.md` and `manifest.json`); the contract
was byte-identical at the end of every run.

| task | calls | check | support | explore | complete | modelling | tool repair | replay | status |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| `t1_same_order` | 1 | valid | supported | PASS | true | faithful | — | — | `already_satisfied` |
| `t2_abba` | 1 | valid | supported | FAIL | true | faithful | repaired | replayed | `repaired` (tool) |
| `t3_cross_module_abba` | 1 | valid | supported | FAIL | true | faithful | repaired | replayed | `repaired` (tool) |

Per-task, reported separately (the four claims are distinct):

- **Static validity**: all three candidates passed `check`; `support` reported
  `supported`.
- **Modelling fidelity** (independent structural check on the frozen CIR):
  - `t1_same_order`: `t1`,`t2` both lock `main::a`→`main::b`; no inversion.
  - `t2_abba`: `t1` locks `main::a`→`main::b`, `t2` locks `main::b`→`main::a` —
    the ABBA inversion is reproduced faithfully (not "fixed" by the model).
  - `t3_cross_module_abba`: modules `main`/`other`; `main::t1` locks
    `main::a`→`other::b`, `other::t2` locks `other::b`→`main::a`; cross-module
    refs declared in `requires.resources`. Faithful.
- **Root verification**: `t1` `PASS` complete (correct model);
  `t2`/`t3` `FAIL` complete (the intended defect is detected).
- **Tool repair** (`repair_source=tool`, not an LLM patch): `t2`/`t3` were
  repaired by the backend's deterministic strategy search under the frozen
  contract; artifacts bound to the invocation and replayed.
- **Replay**: `replayed` for `t2`/`t3`; none for `t1` (already satisfied).

Raw evidence: `runs/<batch>/llm/llm_requests.jsonl` (3 sanitized request/
response records; requested = response = `deepseek-flash`; `thinking` disabled;
`finish_reason=stop`; usage present), the per-run generation candidates, frozen
CIR, per-call CLI stdout/stderr/exit and artifacts.

## 5. Notes and limits

- **No live feedback retry was triggered**: each task's first candidate passed
  static checking, so the model never saw repair feedback. This is reported as
  observed; the feedback path is covered by the offline regression
  (`test_bad_json_then_valid_then_tool_repair`), not by injecting a fake error.
- **`repaired` is a tool result.** The model generated the initial CIR only; the
  repair was the backend strategy search. The external LLM-patch evidence
  protocol is not connected this round, so no "LLM repair" is claimed.
- **Model identity** was Flash for all three responses; no non-Flash identity was
  seen, so no batch stop on identity occurred.
- **Usage** was present for all three calls (prompt/completion/total tokens
  recorded); the code records `null` when absent rather than 0.
- The three tasks are pre-registered modelling probes; this is not a held-out
  evaluation and cannot support generalisation or effectiveness claims.
- ConcIR core was not modified; no new provider, no Pro, no agent framework, no
  large-scale evaluation.

## 6. Deliverables in this directory

- `DEEPSEEK_FLASH_PILOT_HANDOFF.md` (this file), `PROTOCOL.md`, `TASKS.md`,
  `TASKS.json`, `contracts/`.
- `SUMMARY.md`, `SUMMARY.json`, `manifest.json` (code/prompt/contract/binary
  hashes and all evidence file hashes), `test-results.txt`, `live.stdout.json`,
  `live.stderr.txt`.
- `runs/` — the real batch plus the zero-request setup-failure batch: sanitized
  LLM records, generation rounds, frozen CIR, and per-call CLI evidence.

## 7. Next interface work

1. Live multi-round feedback is untested against the real model (it did not
   trigger); a task whose first candidate fails `check` is needed to exercise it.
2. The external patch protocol (LLM-proposed constrained edits entering the
   composite search) remains unconnected; until then, "LLM repair" cannot be
   evidenced.
3. Model/endpoint policy is Flash-only by construction; any future model change
   must be an explicit, reviewed decision, not a fallback.
