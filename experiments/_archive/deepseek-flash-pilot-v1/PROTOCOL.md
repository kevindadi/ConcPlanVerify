# PROTOCOL — deepseek-flash-pilot-v1 (frozen before any model call)

Scope: a minimal, pre-registered engineering pilot of the **generation → check →
bounded feedback retry → freeze → support → explore → tool repair → replay**
loop using the real **DeepSeek Flash** model. This is a link/feasibility check,
not an FSE-scale evaluation, not a claim that the LLM performs constrained patch
repair, and not a statement of general effectiveness.

## Model boundary (hard)

- Provider: `deepseek` only. Model: `deepseek-flash` only. Endpoint:
  `https://api.deepseek.com` (Chat Completions).
- Pro, aliases and provider fallback are forbidden. A non-allowed provider/model
  fails **before** any request (CLI allow-list + client guard), and the response
  `model` is checked against the request after every call; a mismatch stops the
  batch.
- Thinking is explicitly disabled: `extra_body={"thinking": {"type": "disabled"}}`
  (thinking is enabled by default per the DeepSeek docs; omitting the field is
  not sufficient).
- The API key is read from the application repository `.env` via
  `DEEPSEEK_API_KEY`; it is never printed, logged, or written to any record.

## Budget (enforced before/at each real request)

| item | value |
| --- | --- |
| tasks | 3 (below), run in order |
| generation rounds / task | 3 |
| batch request cap | 12 actual HTTP attempts (incl. transport retries) |
| transport retries | at most 1, only for timeout/rate-limit/5xx |
| per request | `max_tokens=4096`, timeout 90 s, `temperature=0.0`, thinking disabled |
| batch wall clock | 20 minutes (shared) |

The budget is persisted (`budget.json`) and counted before each HTTP attempt, so
failed requests/restarts consume it. Non-retryable errors (401/403/402/404/400/422
and invalid model) stop the batch without retry. The first task's first request
is the connectivity check; no separate chat test is made.

## Frozen tasks

Contracts and prompts are frozen before the run. Hashes (`sha256`, first 32):

| item | hash |
| --- | --- |
| TASKS.json | `f21f303e5bb36e4e1fe781075f2ab108` |
| contract t1_same_order | `035d2a951c35e129baf26e3fd4956335` |
| contract t2_abba | `961458b8184169722d788feb17e53f4a` |
| contract t3_cross_module_abba | `a53d94331df2a2502865dde5cbcd8699` |
| generation prompt `concir_generation_v1.md` | `bda51eaaa5cd2c58fe6209d4604f4103` |
| feedback prompt `concir_feedback_v1.md` | `1cdedc80662f9962ee73d2331cb772f2` |
| ConcIR binary | `65a8f633f986e890d2e4256700db3ab6` |

All contracts: `deadlock_free` (`no-deadlock`) + preserved reachable completion
of the two tasks; standard bounds (`max_states=20000`, threads/frames 8, depth 64,
boundary 256); `allowed_scope.allow_lock_reorder=true`. The contract is never
generated or modified by the model, and is verified unchanged at the end of each
run.

| id | modelling task | structural check (independent of the tool) |
| --- | --- | --- |
| `t1_same_order` | two tasks, same two mutexes, one consistent order | `same_order_pair`: ≥2 functions share a pair, **no** inversion, locks balanced |
| `t2_abba` | two tasks acquire the same two mutexes in **opposite** order (existing ABBA) | `abba_inversion`: an inversion is present and locks balanced |
| `t3_cross_module_abba` | two modules share mutexes via `requires.resources` FQN and acquire in opposite order | `cross_module_abba`: inversion across two modules + cross-module resource refs |

## Modelling fidelity vs property verification (kept separate)

The structural check judges whether the generated CIR faithfully represents the
requested pattern. It is independent of the Rust verdict:

- For `t2_abba`/`t3_cross_module_abba`, a model that "fixes" the inversion into a
  same order to obtain `PASS` is recorded as **`modeling_mismatch`**, and the run
  is **not** reported as "requirement met" or as an LLM repair.
- The Rust tool's `FAIL`/`PASS`, `UNKNOWN`, `UNSUPPORTED` and repair outcomes are
  reported as tool facts, separately.

## Repair attribution

After the initial CIR is frozen, the only repair path is the backend's
deterministic strategy search under the frozen contract and its `allowed_scope`;
the result is labelled `repair_source=tool`. The model never rewrites the whole
program after freezing. This round does **not** exercise LLM-proposed constrained
patches (that evidence protocol is not connected).

## Evidence

For every real attempt: sanitized system/user messages, prompt sha256, raw
assistant text, request id, requested/response model, thinking setting,
`finish_reason`, usage (or `null` when absent — never coerced to 0), wall time and
transport-retry index. For every tool call: argv, binary/input/contract hashes,
exit code, stdout/stderr, artifact hash, and the check/support/explore/repair/
replay results with their source labels.

## Stopping

If the budget, model identity, or a non-retryable error stops the batch, the
obtained results and the stop reason are delivered as-is; the batch is not
expanded or switched to another model. Even if a task fails, its records are
kept; only successful results are never shown selectively.
