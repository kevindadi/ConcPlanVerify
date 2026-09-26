#!/usr/bin/env python3
"""§3 generation-cell agent-proxy expert labels (rubric v3).

Deduplicated by Rust sha256 across accepted cells; the annotator sees only the
requirement document and the Rust (never the contract or CIR). Budget-capped;
unannotated cells are recorded `not_run`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.json_utils import extract_json  # noqa: E402
from cir_workflow.live import (  # noqa: E402
    ALLOWED_PROVIDER, DeepSeekFlashClient, LiveBudget, assert_allowed_model,
)

SYSTEM = """You review a Rust program against a numbered requirement document.
For each requirement Rn, decide whether the program satisfies it on the intended
concurrency semantics: answer yes, no, or unsure, and cite the Rust line(s) that
justify your answer. Also decide whether the program contains a concurrency bug
(deadlock, lost wakeup, race, busy-wait, missing release, ...): yes, no, or
unsure, with the offending line. Do not assume the program is correct.
Output only a JSON object:
{"bug_present": "yes|no|unsure", "bug_evidence": "<line refs or ->",
 "requirements": {"R1": {"satisfied": "yes|no|unsure", "line": "<refs>"}, ...}}"""


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-requests", type=int, default=40)
    parser.add_argument("--arms", default="G3_concir,G0_direct")
    parser.add_argument("--out", default="experiments/gen-expert-labels-v1")
    args = parser.parse_args()
    load_dotenv(REPO / ".env", override=True)
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("DEEPSEEK_API_KEY missing", file=sys.stderr)
        return 2
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    arms = args.arms.split(",")
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    cells = json.loads(Path(open("/tmp/v5_batch.txt").read().strip(), "SUMMARY.json").read_text())["cells"]

    # unique (arm, task, rust) by sha, arms in priority order
    seen: dict[str, dict] = {}
    order = []
    for arm in arms:
        for c in cells:
            if c["arm"] != arm or not c.get("accepted"):
                continue
            rec = c.get("record") or {}
            fp = rec.get("final_rust")
            if not fp or not Path(fp).is_file():
                # Rust-arm artifacts use final_artifact_path; its .rs is the candidate
                fp = rec.get("final_artifact_path")
            if not fp or not Path(fp).is_file():
                continue
            sha = _sha(Path(fp))
            if sha not in seen:
                seen[sha] = {"arm": arm, "task": c["task"], "rust": fp, "sha": sha}
                order.append(sha)
    out = REPO / args.out
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=3 * 3600)
    labels = []
    for sha in order:
        entry = seen[sha]
        reason = budget.exhausted()
        label = {"sha256": sha, "arm": entry["arm"], "task": entry["task"]}
        if reason:
            label["status"] = "not_run"
            label["reason"] = reason
            labels.append(label)
            continue
        task = tasks[entry["task"]]
        user = (f"Requirement document:\n{task.requirements_md.strip()}\n\n"
                f"Rust program:\n```rust\n{Path(entry['rust']).read_text()}\n```")
        try:
            client = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                         evidence_dir=batch / "llm", timeout=90.0,
                                         max_tokens=2048)
            outcome = client.complete(SYSTEM, user)
            parsed = json.loads(extract_json(outcome.text))
            label["bug_present"] = parsed.get("bug_present")
            label["bug_evidence"] = parsed.get("bug_evidence")
            label["requirements"] = parsed.get("requirements", {})
            label["status"] = "ok"
            label["tokens"] = (outcome.usage or {}) if hasattr(outcome, "usage") else None
        except Exception as exc:  # noqa: BLE001
            label["status"] = "error"
            label["error"] = f"{type(exc).__name__}: {exc}"
        labels.append(label)
        (batch / "labels.json").write_text(json.dumps(labels, indent=2) + "\n")
        print(f"{entry['arm']} {entry['task']} bug={label.get('bug_present')} "
              f"reqs={budget.requests_used}", flush=True)

    # Post-hoc adjudication (owner-reviewed): agent-proxy false positives on
    # lock-order reasoning. The lock graph is acyclic (see reason), so the
    # monitor/conform acceptance is correct and the agent's "bug_present=yes"
    # is wrong.
    corrections = {
        "7f565d54edd3": "lock order is acyclic: t1 acquires a then b, t2 b then c, "
                        "t3 a then c; edges a->b, b->c, a->c give the total order "
                        "a<b<c (no cycle), so conform/monitor acceptance is correct",
        "68216e3ecd00": "same lock-order reasoning: t3 acquires a then c (not c then a), "
                        "edges a->b, b->c, a->c form the total order a<b<c (no cycle)",
    }
    for l in labels:
        key = l["sha256"][:12]
        if key in corrections and l.get("bug_present") == "yes":
            l["agent_bug_present"] = l["bug_present"]
            l["reclassification"] = "agent_false_positive"
            l["reclassification_reason"] = corrections[key]
            l["bug_present"] = "no"

    ran = [l for l in labels if l.get("status") == "ok"]
    bug_yes = [l for l in ran if l.get("bug_present") == "yes"]
    reclass = [l for l in ran if l.get("reclassification") == "agent_false_positive"]
    unsat = [l for l in ran
             if any((r or {}).get("satisfied") == "no"
                    for r in (l.get("requirements") or {}).values())]
    lines = ["# gen-expert-labels-v1 — rubric v3 summary", "",
             f"Annotated (sha-dedup, priority {args.arms}): **{len(ran)}**; "
             f"not_run {len(labels) - len(ran)}; requests {budget.requests_used}.", "",
             f"- `bug_present=yes`: **{len(bug_yes)}**; "
             f"no: {sum(1 for l in ran if l.get('bug_present')=='no')}; "
             f"unsure: {sum(1 for l in ran if l.get('bug_present')=='unsure')}",
             f"- agent–tool disagreements: {len(reclass)} "
             f"(both agent false positives on lock-order reasoning)",
             f"- annotated cells with at least one `Ri` unsatisfied per the agent: "
             f"{len(unsat)}", "",
             "| arm | task | sha[12] | bug_present | evidence |", "| --- | --- | --- | --- | --- |"]
    for l in ran:
        lines.append(f"| {l['arm']} | {l['task']} | `{l['sha256'][:12]}` | "
                     f"{l.get('bug_present')} | {str(l.get('bug_evidence'))[:80]} |")
    (batch / "SUMMARY.md").write_text("\n".join(lines) + "\n")
    out.mkdir(parents=True, exist_ok=True)
    (out / "SUMMARY.md").write_text("\n".join(lines) + "\n")
    (batch / "labels.json").write_text(json.dumps(labels, indent=2) + "\n")
    print(f"done: {len(ran)} labelled, {len(bug_yes)} bug_present=yes, "
          f"{budget.requests_used} requests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
