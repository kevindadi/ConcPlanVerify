"""Agent-proxy expert labels for every accepted A0/A1/A2 cell of main-v1 (§4.2).

Independent static reading (not the automatic oracle):
- lock-order / structure tasks: reconstruct the lock order from `.lock()` calls
  and flag a cycle;
- condvar tasks: flag `notify_one` with two or more waiters;
- otherwise `unsure`.
Disagreements with the automatic oracle plus a random sample go to the human
review queue (seed recorded; left blank for the owner).
"""

from __future__ import annotations

import hashlib
import json
import random
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.flash_smoke import _bug_sources  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
OUT = REPO / "experiments/flash-repair-main-v1/expert-labels"
RUST_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml")
SEED = 20260920

LOCK_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\.lock\(\)")


def _canon(name: str) -> str:
    name = name.split(".")[-1]
    name = name[4:] if name.startswith("mtx_") else name
    name = re.sub(r"\d+$", "", name)
    return name


def _has_cycle(edges: dict[str, set[str]]) -> bool:
    WHITE, GREY, BLACK = 0, 1, 2
    color = {n: WHITE for n in edges}
    for node in list(edges):
        stack = [(node, iter(sorted(edges.get(node, ()))))]
        if color[node] != WHITE:
            continue
        color[node] = GREY
        while stack:
            cur, it = stack[-1]
            advanced = False
            for nxt in it:
                if color.get(nxt, WHITE) == GREY:
                    return True
                if color.get(nxt, WHITE) == WHITE:
                    color[nxt] = GREY
                    stack.append((nxt, iter(sorted(edges.get(nxt, ())))))
                    advanced = True
                    break
            if not advanced:
                color[cur] = BLACK
                stack.pop()
    return False


def _spawn_bodies(src: str) -> list[str]:
    bodies = []
    for match in re.finditer(r"spawn\s*\(\s*(?:move\s*)?\|", src):
        brace = src.find("{", match.end())
        if brace < 0:
            continue
        depth, i = 0, brace
        while i < len(src):
            if src[i] == "{":
                depth += 1
            elif src[i] == "}":
                depth -= 1
                if depth == 0:
                    bodies.append(src[brace:i + 1])
                    break
            i += 1
    return bodies


def _body_lock_edges(body: str) -> dict[str, set[str]]:
    """Lock-order edges within one thread body, ignoring a guard dropped first."""

    events = []
    guard = None
    for i, line in enumerate(body.splitlines(), 1):
        m = re.search(r"let\s+(?:mut\s+)?(\w+)\s*=\s*([A-Za-z_][\w.]*)\.lock\(\)", line)
        if m:
            guard = m.group(1)
            events.append(("lock", i, _canon(m.group(2)), guard))
            continue
        m = re.search(r"([A-Za-z_][\w.]*)\.lock\(\)", line)
        if m:
            events.append(("lock", i, _canon(m.group(1)), None))
            continue
        m = re.search(r"drop\(\s*(\w+)\s*\)", line)
        if m:
            events.append(("drop", i, m.group(1), None))
    edges: dict[str, set[str]] = {}
    prev = None  # (mutex, guard)
    for kind, _line, value, g in events:
        if kind == "drop":
            if prev and prev[1] == value:
                prev = None
            continue
        if prev and prev[0] != value:
            edges.setdefault(prev[0], set()).add(value)
        prev = (value, g)
    return edges


def lock_order_label(source: str) -> tuple[str, list[int], str]:
    bodies = _spawn_bodies(source)
    if not bodies:
        return "unsure", [], "no spawn closure found"
    edges: dict[str, set[str]] = {}
    for body in bodies:
        for a, targets in _body_lock_edges(body).items():
            edges.setdefault(a, set()).update(targets)
    if not edges:
        return "unsure", [], "no nested lock acquisitions found"
    if _has_cycle(edges):
        return "yes", [], f"lock-order cycle among {sorted(edges)}"
    return "no", [], "no lock-order cycle in the observed acquisition order"


def condvar_label(source: str) -> tuple[str, list[int], str]:
    waiters = len(re.findall(r"\.wait(_while)?\(", source))
    waiter_bodies = sum(1 for b in _spawn_bodies(source) if ".wait(" in b or ".wait_while(" in b)
    if re.search(r"for\s+_\s+in\s+0\s*\.\.\s*[2-9]", source):
        waiter_bodies = max(waiter_bodies, 2)
    if "notify_one" in source and max(waiters, waiter_bodies) >= 2:
        lines = [i for i, l in enumerate(source.splitlines(), 1) if "notify_one" in l]
        return "yes", lines, f"notify_one with {waiters} waiters"
    if "notify_all" in source and waiters >= 2:
        return "no", [], f"notify_all wakes all {waiters} waiters"
    return "unsure", [], "no clear condvar defect"


def static_label(task: str, source: str) -> tuple[str, list[int], str, bool]:
    if task.startswith("lock-order/") or task.startswith("structure/"):
        bug, lines, why = lock_order_label(source)
        # design_preserved: two independent mutexes retained?
        preserved = source.count("Mutex::new") >= 2 or "Semaphore" in source
        return bug, lines, why, preserved
    if task.startswith("condvar/"):
        bug, lines, why = condvar_label(source)
        return bug, lines, why, "Condvar::new" in source
    return "unsure", [], "no static rule for this family", True


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    candidates = []
    for rep in summary["reps"]:
        for task in rep["tasks"]:
            tid = task["task"]
            for arm in RUST_ARMS:
                rec = (task.get("arms") or {}).get(arm) or {}
                if not rec.get("accepted"):
                    continue
                path = rec.get("final_artifact_path")
                if not path or not Path(path).is_file():
                    continue
                text = Path(path).read_text(encoding="utf-8")
                bug, lines, why, preserved = static_label(tid, text)
                auto = bool(_bug_sources(rec))
                candidates.append({
                    "task": tid, "arm": arm, "rep": rep["rep"],
                    "path": str(Path(path).relative_to(BATCH)),
                    "sha256": hashlib.sha256(text.encode()).hexdigest(),
                    "bug_present": bug, "defect_lines": lines,
                    "design_preserved": "yes" if preserved else "no",
                    "reason": why, "auto_bug_present": auto,
                    "labeler": "agent-proxy", "rubric_version": "expert-label-rubric-v1",
                    "human_reviewed": False, "human_label": None,
                })
    (OUT / "CANDIDATES.json").write_text(json.dumps(candidates, indent=1) + "\n",
                                         encoding="utf-8")
    compared = [c for c in candidates if c["bug_present"] != "unsure"]
    agree = sum(1 for c in compared if (c["bug_present"] == "yes") == c["auto_bug_present"])
    disagreements = [c for c in compared if (c["bug_present"] == "yes") != c["auto_bug_present"]]
    rng = random.Random(SEED)
    pool = [c for c in candidates if c not in disagreements]
    sample = rng.sample(pool, min(4, len(pool)))
    queue = disagreements + sample
    lines_md = ["# Expert labels — main-v1 accepted Rust cells (agent-proxy)", "",
                f"- rubric: `expert-label-rubric-v1`; seed `{SEED}`",
                f"- candidates: {len(candidates)}; compared {len(compared)}; "
                f"agreement {agree}/{len(compared)} = "
                f"{(agree / len(compared)) if compared else 0:.3f}; "
                f"unsure {len(candidates) - len(compared)}", "",
                "| task | arm | rep | bug_present | design_preserved | auto | agree |",
                "| --- | --- | --- | --- | --- | --- | --- |"]
    for c in candidates:
        match = ("—" if c["bug_present"] == "unsure"
                 else str((c["bug_present"] == "yes") == c["auto_bug_present"]))
        lines_md.append(f"| {c['task']} | {c['arm']} | {c['rep']} | {c['bug_present']} | "
                        f"{c['design_preserved']} | {c['auto_bug_present']} | {match} |")
    (OUT / "EXPERT_LABELS.md").write_text("\n".join(lines_md) + "\n", encoding="utf-8")
    queue_md = ["# Human review queue (leave blank for the owner)", "",
                f"All expert/automatic disagreements plus a random sample (seed `{SEED}`).",
                "Fill `human_label` (`yes`/`no`/`unsure`) and one reason; do not let an",
                "agent fill it.", "",
                "| task | arm | rep | sha256 | agent bug_present | auto | human_label | reason |",
                "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for c in queue:
        queue_md.append(f"| {c['task']} | {c['arm']} | {c['rep']} | `{c['sha256'][:12]}` | "
                        f"{c['bug_present']} | {c['auto_bug_present']} | | |")
    (OUT / "HUMAN_REVIEW_QUEUE.md").write_text("\n".join(queue_md) + "\n", encoding="utf-8")
    # Machine-readable, aggregated per (task, arm) for the RESULTS generator.
    agg: dict[str, dict] = {}
    for c in candidates:
        key = f"{c['task']}|{c['arm']}"
        slot = agg.setdefault(key, {"task": c["task"], "arm": c["arm"],
                                    "bug_present": "no", "design_preserved": "yes",
                                    "reps": 0, "labeler": "agent-proxy"})
        slot["reps"] += 1
        if c["bug_present"] == "yes":
            slot["bug_present"] = "yes"
        elif c["bug_present"] == "unsure" and slot["bug_present"] == "no":
            slot["bug_present"] = "unsure"
        if c["design_preserved"] == "no":
            slot["design_preserved"] = "no"
    payload = {"rubric_version": "expert-label-rubric-v1",
               "labeler": "agent-proxy", "seed": SEED,
               "agreement": {"compared": len(compared), "agree": agree},
               "labels": list(agg.values())}
    (OUT / "EXPERT_LABELS.json").write_text(json.dumps(payload, indent=1) + "\n",
                                            encoding="utf-8")
    print(json.dumps({"candidates": len(candidates), "compared": len(compared),
                      "agreement": f"{agree}/{len(compared)}",
                      "disagreements": len(disagreements),
                      "human_queue": len(queue)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
