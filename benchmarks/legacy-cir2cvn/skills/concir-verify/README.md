# concir-verify skill

A standalone, agent-agnostic skill that teaches an LLM how to **verify,
generate, and repair** concurrent-program models written in
[ConcIR](../../doc/syntax.md), using the `cir2cvn` -> CVN analysis in this
repository.

## Layout

```
skills/concir-verify/
├── SKILL.md                         # the instruction file (load this)
├── README.md                        # this file
├── reference/
│   └── verification-contract.md     # JSON output contract + error-code catalog
└── scripts/
    ├── build.sh                     # build Rust binary + Python venv
    ├── validate.sh                  # cir2cvn --validate
    ├── analyze.sh                   # cir2cvn --analyze
    ├── goals.sh                     # cir2cvn --goals
    └── workflow.sh                  # python -m cir_workflow generate/repair/plan/merge
```

All scripts are shell; they auto-locate the repo root (env
`CONCPLANVERIFY_ROOT` overrides, otherwise they walk up from the script
location) and build the binary on first use.

## What the LLM decides vs. what the tools decide

- **Tools (deterministic)**: validate/analyze/explore — fixed semantics, one
  input one verdict.
- **LLM (this skill)**: when to run which tool, how to interpret the JSON
  verdict, when to repair, when to ask the user, when to stop.

## Installing into an agent

- **Claude Code**: symlink or copy to `~/.claude/skills/concir-verify/`
  (Claude auto-loads `~/.claude/skills/<name>/SKILL.md`).
- **opencode**: copy to `.opencode/skills/concir-verify/` in this repo (or add
  the path to `skills.paths` in `opencode.json`).
- **Any other agent**: point it at `SKILL.md` as the instruction file.

The scripts work from any location because they locate the repo root at
runtime; keep the repo checked out and the tools built.

## Smoke test

```bash
# From the repo root:
bash skills/concir-verify/scripts/analyze.sh tests/e2e/mutex_deadlock/buggy.json  # verified_unsafe
bash skills/concir-verify/scripts/analyze.sh tests/e2e/mutex_deadlock/fixed.json  # verified_safe
bash skills/concir-verify/scripts/validate.sh tests/fixtures/canonical_schema.json # valid

# From skills/concir-verify/ (paths are relative to the repo root, not the skill dir):
bash scripts/analyze.sh ../../tests/e2e/mutex_deadlock/buggy.json
```

The scripts locate the repo root themselves (`CONCPLANVERIFY_ROOT` overrides);
input file paths are resolved by the binary relative to the **current working
directory**, so pass paths as you would from wherever you run them.
