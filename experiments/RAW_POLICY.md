# RAW_POLICY — tracked evidence vs on-disk raw tier

The frozen `experiments-v2-freeze-*` tags keep a complete copy of every run.
To keep the working tree reviewable, only the **evidence tier** is tracked;
the **raw tier** stays on disk (and in the tags) but is git-ignored for new
runs and untracked for existing ones.

## Evidence tier (tracked)

- `PROTOCOL.md`, `SUMMARY.md`, `SUMMARY.json`, `CELLS.json`, `CELL.json`
- per-round and final `.rs` and `.cir.json` artifacts
- `mapping.json`
- LLM round-trips `reply.json` / `llm-*.json` (prompt + answer; scanned for secrets)
- conform/monitor verdicts `stdout.json`, `argv.json`
- expert labels and `HUMAN_REVIEW_QUEUE*.md`
- `FAILURES.md`, `CONFORM_GAPS.md`, case notes

## Raw tier (on disk + tags only; excluded from git and from the supplement)

- `native-*.jsonl`, `miri-*.jsonl` trace sets
- `traces/`, `monitor-traces/`, `conform-traces/`
- `proj/` (`Cargo.toml`, `Cargo.lock`, `src/main.rs` copies)
- `cir_trace.rs`, `concir_sync.rs` copies
- `env.json`, `wall_ms.txt`, `exit.txt`, `build.stdout.txt`, `build.stderr.txt`
- empty `stderr.txt`

## Retrieval

`git checkout <freeze-tag> -- <path>` restores a raw file. `make supplement
--with-traces` (future) packages the raw tier as a separate zip. The evidence
tier is self-sufficient for regenerating `RESULTS.md` and `tables/*.tex`;
`make verify-evidence` checks this by archiving `HEAD` and re-running
`python -m cir_workflow results`.
