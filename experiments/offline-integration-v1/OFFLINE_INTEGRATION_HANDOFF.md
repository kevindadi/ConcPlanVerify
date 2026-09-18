# OFFLINE_INTEGRATION_HANDOFF

Round: migrate the retired `cir2cvn` integration to the current ConcIR
`concir-backend` CLI, build an offline Python workflow around a scripted
provider, and fix the remaining runner-ledger items. No real LLM API was called;
no credentials were read; no CIR/Petri/verification/repair semantics were
re-implemented in Python.

Repository responsibilities (see `../../REPOSITORY_BOUNDARIES.md`):

- `/Users/kevin/local-repos/ConcPlanVerify` — Python app (this work).
- `/Users/kevin/local-repos/ConcIR` — Rust CLI (only small runner-ledger fixes).
- `/Users/kevin/paper-review/papers/ConcPlanVerify` — this evidence directory.

## 1. What is implemented

| component | path | state |
| --- | --- | --- |
| Typed CLI client | `python/cir_workflow/concir_client.py` | check/support/explore/repair(flag)/replay; per-call dir; typed exit/JSON mapping; process-group timeout; binary/input/contract identity |
| Providers | `python/cir_workflow/providers.py` | `CandidateProvider` protocol, `ScriptedProvider`, `LlmCandidateProvider` (library only) |
| Offline workflow | `python/cir_workflow/offline_workflow.py` | generate → check → bounded feedback retry → freeze → support → explore → tool repair → replay |
| Prompts | `python/cir_workflow/prompts.py`, `prompt_assets/concir_{generation,feedback}_v1.md` | modular-CIR generation + structured feedback; sha256 recorded |
| CLI entry | `python/cir_workflow/__main__.py` | `check`/`support`/`explore`/`repair`/`replay`/`offline` |
| Tests | `python/tests/` | 29 tests: protocol unit tests + real-CLI integration + offline workflow |

Workflow guarantees enforced in code:

- the contract is caller-supplied and frozen (sha256 checked at the end);
- provider retries are bounded and only triggered by parse/`check` failure;
- once the initial CIR passes `check` it is frozen, and the only repair path is
  the backend strategy search under the frozen contract/`allowed_scope`;
- a repair is labelled source `tool`, never `LLM`;
- `UNKNOWN`/`UNSUPPORTED`/process errors stop the run and are never reported as
  safe;
- a `repaired` result requires the artifact to bind to this invocation
  (`effective_config` + `explore` fingerprints) and to pass `replay`.

## 2. Migration

See `REPO_MIGRATION.md`. In short: the `cir2cvn` Rust crate (translator,
verification, repair), the old `--validate/--analyze/--goals` client, the
flat-schema prompts/commands, the old Rust tests and the old benchmark/docs/UI
were removed from the app repository; the valuable history was migrated to
`legacy-cir2cvn/`. Real-provider plumbing (`llm.py`) is kept but not wired to a
network command. The migration was recorded in the app repository by its owner
as commits `31f905e` and `c8322ae`; `REPO_MIGRATION.md` lists one follow-up
(the stale `unipn` gitlink).

## 3. CLI contract

See `CLI_CONTRACT.md` for commands, exit codes, payload fields, client-side
failure classes, artifact binding, and the known gaps (legacy positional repair,
external-patch protocol, RwLock).

## 4. Tests and demo

Test command and result (`test-results.txt`):

```bash
cd /Users/kevin/local-repos/ConcPlanVerify
PYTHONPATH=python:. python3 -m unittest discover -s python/tests -t .
# Ran 29 tests ... OK
```

Breakdown: 9 unit (stub binary: exit/JSON mapping, empty/non-JSON, deadline,
paths with spaces, missing binary, raw-output capture), 5 integration (real
`concir-backend`: check, support, all explore outcomes, repair+replay, tampered
replay), 7 offline (feedback retry, already satisfied, UNKNOWN, UNSUPPORTED,
contract frozen, finite retry budget, non-repair outcome), plus the retained
provider/helper tests.

Offline demos (scripted provider, real CLI; raw records in `demo/<name>/` with
`report.json`, `cli.stdout.json`, and per-call `out/*/{stdout.json,stderr.txt,exit.txt}`):

| demo | scripted turns | result | steps |
| --- | --- | --- | --- |
| `retry` | bad JSON, then valid CIR | `repaired` | generate[1] scripted parse_error → check[2] valid → support → explore FAIL → repair repaired → replay replayed |
| `already` | already-correct CIR | `already_satisfied` | check valid → support → explore PASS (no repair) |
| `unknown` | bounded-bounds CIR | `unknown` | explore UNKNOWN, stop |
| `unsupported` | RwLock CIR | `unsupported` | support unsupported (8 `rwlock` locations), stop |
| `nonrepair` | repairable CIR, candidate_budget 0 | `budget_exhausted` | repair budget_exhausted, no success claimed |

This demonstrates every required offline case: feedback retry was actually used,
the already-correct model never called repair, a FAIL was fixed by the tool and
replayed, and UNKNOWN/UNSUPPORTED/budget stop with their real status.

## 5. Verified with the real CLI vs interface only

Verified end to end against the real binary (`concir-backend`, sha256
`65a8f633f986e890…`):

- `check`, `support`, `explore` (PASS/FAIL/UNKNOWN/INVALID/UNSUPPORTED),
  `repair` (flag strategy mode) with artifact, `replay` success and tamper
  failure;
- the whole offline loop above, including frozen-contract equality and artifact
  binding.

Interface only (no network call this round):

- `LlmCandidateProvider` / `llm.py` DeepSeek/Qwen adapters — exercised only with
  fake SDK clients (`test_llm.py`), never against an API;
- the `offline` command's `--provider` accepts `scripted` only.

## 6. Interface gaps for the next round

1. **Real LLM wiring.** `LlmCandidateProvider` needs a `generate` entry point
   (and API-key handling) after an explicit go-ahead; prompts are already
   modular-CIR and versioned.
2. **External-patch evidence protocol.** There is no supported path for
   LLM-proposed edits to enter the composite search and yield a replayable
   artifact; the legacy positional `repair` output must not be presented as one.
   This needs a backend protocol decision before LLM repair can be evidence.
3. **Repair scope.** The offline loop uses backend-generated repairs only; the
   provider cannot propose a whole-program replacement after freezing.

## 7. Boundaries honored

- No changes to ConcIR core semantics; only the small runner-ledger fixes in
  `REAL_CASES_V0_ERRATA.md`.
- No LLM/provider code added to ConcIR; no network requests; no credentials read.
- Old ConcIR pilot data unchanged (frozen checksum still matches).
- Artifacts/demo raw files and hashes are in `manifest.json`.
