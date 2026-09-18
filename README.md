# ConcPlanVerify — Python workflow

Python orchestration for LLM-driven ConcIR generation and repair. This package
owns model interaction, prompts, and the generation/repair workflow. It calls the
Rust **`concir-backend`** CLI for all formal work: CIR validation, supportability,
exploration/verification, diagnosis, deterministic repair and artifact replay.
It never re-implements CIR semantics, Petri translation, verification, candidate
enumeration, patch legality or repair acceptance.

> The old `cir2cvn` translator, its `--validate/--analyze/--goals` protocol and
> the flat `resources`/`functions`/`op`/`transfer` schema are retired. See the
> migration report in the paper repository:
> `experiments/offline-integration-v1/REPO_MIGRATION.md`.

## Layout

```
python/cir_workflow/
├── concir_client.py     # typed subprocess client for concir-backend
├── providers.py         # CandidateProvider protocol + ScriptedProvider (+ LLM adapter)
├── offline_workflow.py  # generation -> check -> feedback -> verify -> tool repair
├── prompts.py           # versioned prompt assets + structured feedback
├── prompt_assets/       # modular-CIR generation / feedback prompt text (v1)
├── llm.py               # DeepSeek/Qwen SDK adapters (library only; not auto-run)
├── models.py            # ModelConfig + token-usage helpers
├── env.py, json_utils.py
└── __main__.py          # CLI entry point
```

## Prerequisites

- A built ConcIR CLI: in the ConcIR repo run `cargo build --release --bin concir-backend`.
- Python 3.10+. The workflow itself has no required third-party dependency;
  `openai` is only needed for real LLM providers (not used offline).

Point the client at the binary with `--binary` or `CONCIR_BACKEND`.

## CLI usage

Thin wrappers over the current backend (the raw JSON is printed to stdout; the
process exit code mirrors the backend's semantic exit for these commands):

```bash
export CONCIR_BACKEND=/path/to/ConcIR/target/release/concir-backend
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out check   program.json
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out support program.json
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out explore program.json contract.json petri
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out repair  program.json contract.json --strategy c
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out replay  artifact.json
```

Offline workflow with the scripted provider (no API key, no network):

```bash
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-offline offline \
  --requirements "Two tasks take the same two locks in opposite order." \
  --contract contract.json --provider scripted --script responses.json
```

`responses.json` is a list of `{"text": "<CIR JSON>"}` (or `{"error": "..."}`)
turns replayed in order.

## Workflow guarantees

- The contract is supplied by the caller and frozen; the provider never writes
  or edits it.
- Generation candidates are retried only on parse/`check` failure, within a
  bounded number of rounds. Once a candidate passes static checking the initial
  CIR is frozen.
- After freezing, the only repair path is the backend's deterministic strategy
  search under the frozen contract and its `allowed_scope`; the provider cannot
  replace the whole program to manufacture a pass.
- A tool repair is labelled `tool`, never `LLM`. `UNKNOWN`/`UNSUPPORTED`/tool
  errors are reported as such, never as "safe".
- Every complete repair artifact is replayed by the backend before it is treated
  as evidence.

## Tests

```bash
PYTHONPATH=python:. python3 -m unittest discover -s python/tests -t .
```

Unit tests exercise the protocol mapping with a controllable stub; integration
tests (`test_concir_integration.py`, `test_offline_workflow.py`) call the real
`concir-backend` binary and skip with a clear message when it is absent.

## Roadmap

Real LLM providers (DeepSeek/Qwen) are implemented in `llm.py` and exposed
through `LlmCandidateProvider`, but are not wired to a network command in this
round. The external-provider *patch* protocol (LLM-proposed edits entering the
composite search) is a known gap; this round closes the strategy-mode loop only.
