# Experiments

## Environment

- Rust scaling and goal ablation: Apple M4 Pro, 24 GB, release build.
- LLM supplements: DeepSeek V4 Flash, temperature 0, high reasoning, thinking enabled, one run
  per input/condition.
- Verifier search cap: 100,000 states.

## Commands

```bash
cargo run --release --example rebuttal_experiments -- \
  goal-ablation \
  --output paper/rebuttal-experiments/goal-ablation.json

cargo run --release --example rebuttal_experiments -- \
  scaling \
  --output paper/rebuttal-experiments/scaling.json

PYTHONPATH=python python/.venv/bin/python \
  paper/rebuttal-experiments/run_diagnostic_ablation.py \
  --output paper/rebuttal-experiments/diagnostic-ablation-hard.json

PYTHONPATH=python python/.venv/bin/python \
  paper/rebuttal-experiments/run_paraphrase_robustness.py \
  --output paper/rebuttal-experiments/paraphrase-robustness.json
```

The LLM scripts require the provider credentials configured by the repository's
existing `.env` workflow. Use `--resume` to skip completed diagnostic tasks. In
the paraphrase script, incomplete tasks are retried on resume.

In the diagnostic result files, `self_repair` is the legacy internal key for
the **Unguided** condition: it only says that the CIR needs repair. The
`coarse` key is reported as the **LLM-only** issue-report baseline: it gives the
primary problem class and a generic class-level description, but no SID,
resource/function localization, witness, state summary, or CIR slice.

## Result files

- `goal-ablation.json`: full verifier versus goals disabled, including the
  controlled behavior-deletion mutation.
- `scaling.json`: parameterized lock rings and independent condition-variable
  handshakes; cap-row timings measure time until abort.
- `diagnostic-ablation.json`: prompts, complete candidates, verifier payloads,
  preservation checks, and token usage for the saturated three-case pilot.
- `diagnostic-hard-preregistration.md`: protocol frozen before the hard rerun.
- `diagnostic-hard-fixtures/`: four initial/reference task pairs. References
  establish that a strictly preserving safe repair exists and are not prompted.
- `diagnostic-ablation-hard.json`: all 12 hard-run cells, including complete
  prompts/candidates, strict acceptance checks, verifier payloads, and usage.
  All accepted candidates were additionally rechecked with exact `entry` and
  `fn_summaries` preservation; all 12 still pass.
- `paraphrase-robustness.json`: newly authored NL specifications, all generation
  attempts, validator feedback, and semantic-oracle checks for all nine inputs.

The JSON result files intentionally retain complete prompts and verifier
payloads for auditability and are therefore large.
