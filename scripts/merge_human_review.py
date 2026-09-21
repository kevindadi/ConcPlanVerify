"""§0 merge the owner's human review into EXPERT_LABELS.json.

Reads HUMAN_REVIEW_QUEUE.md (human_label filled, reason blank), attaches
`human_label` / `human_reviewed` / `human_reason: null` per candidate sha
(consistent across rows), and reports human-vs-agent and human-vs-auto
agreement. No LLM requests; the queue is never written by this script.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
LABELS = REPO / "experiments/flash-repair-main-v1/expert-labels/EXPERT_LABELS.json"
QUEUE = REPO / "experiments/flash-repair-main-v1/expert-labels/HUMAN_REVIEW_QUEUE.md"


def parse_queue() -> dict[str, str]:
    by_sha: dict[str, set[str]] = {}
    for line in QUEUE.read_text(encoding="utf-8").splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 7 or cells[0] == "task" or set(cells[0]) <= set("-: "):
            continue
        sha = cells[3].strip("`")
        human = cells[6]
        if human not in ("yes", "no", "unsure"):
            continue
        by_sha.setdefault(sha, set()).add(human)
    out = {}
    for sha, values in by_sha.items():
        if len(values) != 1:
            raise SystemExit(f"inconsistent human_label for sha {sha}: {values}")
        out[sha] = values.pop()
    return out


def main() -> int:
    human = parse_queue()
    payload = json.loads(LABELS.read_text(encoding="utf-8"))
    merged = 0
    for item in payload["labels"]:
        sha12 = item["sha256"][:12]
        if sha12 in human:
            item["human_label"] = human[sha12]
            item["human_reviewed"] = True
            item["human_reason"] = None
            merged += 1
    agent_cmp = agent_agree = auto_cmp = auto_agree = 0
    agent_unsure = 0
    auto_disagree = []
    sys.path.insert(0, str(REPO / "python"))
    from cir_workflow.flash_smoke import _bug_sources  # noqa: E402
    batch = json.loads((REPO / "experiments/flash-repair-main-v1/"
                        "run-20260920T083344-41096-8172c5/SUMMARY.json").read_text())
    auto_map = {}
    for rep in batch["reps"]:
        for task in rep["tasks"]:
            for arm, rec in (task.get("arms") or {}).items():
                auto_map[f"{task['task']}|{arm}|{rep['rep']}"] = bool(
                    rec.get("accepted") and _bug_sources(rec))
    for item in payload["labels"]:
        if not item.get("human_reviewed"):
            continue
        h = item["human_label"]
        a = item.get("bug_present")
        if h == "unsure":
            continue
        if a == "unsure":
            agent_unsure += 1
        else:
            agent_cmp += 1
            agent_agree += 1 if (h == "yes") == (a == "yes") else 0
        cells = item.get("cells", []) or []
        auto_vals = [auto_map.get(f"{c['task']}|{c['arm']}|{c['rep']}") for c in cells]
        auto_vals = [v for v in auto_vals if v is not None]
        auto = any(auto_vals) if auto_vals else None
        if auto is not None:
            auto_cmp += 1
            match = (h == "yes") == bool(auto)
            auto_agree += 1 if match else 0
            if not match:
                auto_disagree.append({"sha256": item["sha256"], "task": item["task"],
                                      "arm": item["arm"], "human": h, "auto": auto})
    payload["human_review"] = {
        "merged_rows": len(human), "candidates": merged,
        "agent_vs_human": {"compared": agent_cmp, "agree": agent_agree,
                           "agent_unsure": agent_unsure},
        "auto_vs_human": {"compared": auto_cmp, "agree": auto_agree,
                          "disagreements": auto_disagree},
    }
    LABELS.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(payload["human_review"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
