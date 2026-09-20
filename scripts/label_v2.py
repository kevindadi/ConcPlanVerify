"""Expert labels v2 (§4): per-candidate, with evidence, for every accepted
Rust candidate of flash-repair-main-v1 plus the §3 A3 Rust products.

Independent static reading (agent-proxy), deduplicated by candidate sha256.
`unsure` carries `unsure_reason ∈ {needs_execution, unfamiliar_api,
ambiguous_spec}`. `design_preserved` is judged against the original design
(requirements + frozen contract's `preserved`).
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

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
A3_RUN = sorted((REPO / "experiments/a3-to-rust-v1").glob("run-*"))[-1]
OUT = REPO / "experiments/flash-repair-main-v1/expert-labels"
RUST_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml")
SEED = 20260920
LOCK_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\.lock\(\)")
CHAN_RE = re.compile(r"\.(send|recv)\(")
ACQ_RE = re.compile(r"\.(acq|acquire)\(")
REL_RE = re.compile(r"\.(rel|release)\(")


def _canon(name: str) -> str:
    name = name.split(".")[-1]
    name = name[4:] if name.startswith("mtx_") else name
    return re.sub(r"\d+$", "", name)


def _spawn_bodies(src: str) -> list[str]:
    bodies = []
    for m in re.finditer(r"spawn\s*\(\s*(?:move\s*)?\|", src):
        brace = src.find("{", m.end())
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


def _function_bodies(src: str) -> list[str]:
    bodies = []
    for m in re.finditer(r"\bfn\s+\w+\s*(?:<[^>]*>)?\s*\([^)]*\)[^{;]*\{", src):
        brace = m.end() - 1
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


def _has_cycle(edges: dict[str, set[str]]) -> bool:
    color: dict[str, int] = {}

    def dfs(n: str) -> bool:
        color[n] = 1
        for m in edges.get(n, ()):
            if color.get(m) == 1:
                return True
            if color.get(m, 0) == 0 and dfs(m):
                return True
        color[n] = 2
        return False

    return any(color.get(n, 0) == 0 and dfs(n) for n in list(edges))


def _lock_events(body: str):
    events, guard = [], None
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
    return events


def _lock_analysis(src: str):
    bodies = _spawn_bodies(src)
    if not any(LOCK_RE.search(b) for b in bodies):
        # Codegen skeletons put the locks in `fn cf_*` functions rather than in
        # inline spawn closures.
        bodies = _function_bodies(src)
    edges: dict[str, set[str]] = {}
    nested = False
    evidence = []
    for b in bodies:
        seq, prev = [], None
        for kind, line, value, guard in _lock_events(b):
            if kind == "drop":
                if prev and prev[1] == value:
                    prev = None
                continue
            seq.append(value)
            if prev:
                edges.setdefault(prev[0], set()).add(value)
                nested = True
            prev = (value, guard)
        if seq:
            evidence.append("->".join(seq))
    return edges, nested, evidence, bodies


def label(source: str, task: str):
    """Return (bug_present, evidence, design_preserved, unsure_reason)."""
    if task.startswith("lock-order/") or task.startswith("structure/"):
        edges, nested, seqs, bodies = _lock_analysis(source)
        ev = "lock sequences: " + ("; ".join(seqs) if seqs else "none")
        if not seqs:
            return "unsure", ev, "no", "unfamiliar_api"
        if _has_cycle(edges):
            return "yes", ev + f"; cycle among {sorted(edges)}", "yes", None
        # Locks exist but form no cycle. The design is preserved only if locks
        # are still held nested (the contract's `holds_all`).
        preserved = "yes" if nested else "no"
        return "no", ev + "; no cycle", preserved, None
    if task.startswith("condvar/"):
        waiters = len(re.findall(r"\.wait(_while)?\(", source))
        bodies = sum(1 for b in _spawn_bodies(source) if ".wait(" in b or ".wait_while(" in b)
        if re.search(r"for\s+_\s+in\s+0\s*\.\.\s*[2-9]", source):
            bodies = max(bodies, 2)
        ev = f"waiters={max(waiters, bodies)}; notify_one={'yes' if 'notify_one' in source else 'no'}; notify_all={'yes' if 'notify_all' in source else 'no'}"
        if "notify_all" in source and max(waiters, bodies) >= 2:
            return "no", ev, "yes", None
        if "notify_one" in source and max(waiters, bodies) >= 2:
            return "yes", ev, "yes", None
        return "unsure", ev, "yes" if "Condvar::new" in source else "no", "ambiguous_spec"
    if task.startswith("channel/"):
        held = []
        bodies = _spawn_bodies(source)
        if not bodies:
            bodies = _function_bodies(source)
        for b in bodies:
            guards: dict[str, int] = {}
            depth = 0
            for i, line in enumerate(b.splitlines(), 1):
                if CHAN_RE.search(line) and any(depth <= d for d in guards.values()):
                    held.append(i)
                m = re.search(r"let\s+(?:mut\s+)?(\w+)\s*=\s*[\w.]+\.lock\(\)", line)
                if m:
                    guards[m.group(1)] = depth
                for g in list(guards):
                    if re.search(rf"drop\(\s*{g}\s*\)", line):
                        guards.pop(g, None)
                depth += line.count("{") - line.count("}")
                guards = {g: d for g, d in guards.items() if d <= depth}
        ev = f"channel ops with a lock held: {held or 'none'}"
        if held:
            return "yes", ev, "yes", None
        return "no", ev, "yes" if ("Mutex::new" in source and ("channel" in source or "mpsc" in source)) else "no", None
    if task.startswith("semaphore/"):
        ev = []
        for b in _spawn_bodies(source):
            a = len(ACQ_RE.findall(b))
            r = len(REL_RE.findall(b))
            ev.append(f"acq={a},rel={r}")
            if a != r:
                return "yes", "; ".join(ev), "yes", None
        return "no", "; ".join(ev) or "no acquire/release found", "yes", None
    return "unsure", "no static rule for this family", "yes", "unfamiliar_api"


def _candidates():
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    out: dict[str, dict] = {}
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
                sha = hashlib.sha256(text.encode()).hexdigest()
                slot = out.setdefault(sha, {"task": tid, "arm": arm, "kind": "rust",
                                            "text": text, "cells": []})
                slot["cells"].append({"task": tid, "arm": arm, "rep": rep["rep"]})
            for arm in ("A3_local", "A3_whole"):
                rec = (task.get("arms") or {}).get(arm) or {}
                if not rec.get("accepted"):
                    continue
                cir = rec.get("final_cir")
                if not cir or not Path(cir).is_file():
                    continue
                cir_sha = hashlib.sha256(Path(cir).read_bytes()).hexdigest()
                skel = (A3_RUN / f"{tid.replace('/', '__')}__{arm}__{cir_sha[:12]}"
                        / "skeleton" / "src" / "main.rs")
                if not skel.is_file():
                    continue
                text = skel.read_text(encoding="utf-8")
                sha = hashlib.sha256(text.encode()).hexdigest()
                slot = out.setdefault(sha, {"task": tid, "arm": arm, "kind": "a3-rust",
                                            "text": text, "cells": []})
                slot["cells"].append({"task": tid, "arm": arm, "rep": rep["rep"]})
    return out


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    cands = _candidates()
    labels = []
    for sha, slot in cands.items():
        bug, ev, preserved, unsure = label(slot["text"], slot["task"])
        labels.append({
            "sha256": sha, "kind": slot["kind"], "task": slot["task"], "arm": slot["arm"],
            "bug_present": bug, "evidence": ev, "design_preserved": preserved,
            "unsure_reason": unsure, "cells": slot["cells"],
            "labeler": "agent-proxy", "rubric_version": "expert-label-rubric-v2",
        })
    compared = [x for x in labels if x["bug_present"] != "unsure"]
    unsure = [x for x in labels if x["bug_present"] == "unsure"]
    disagree = []
    # Agreement against the automatic oracle at cell level (false_accept).
    from cir_workflow.flash_smoke import _bug_sources
    auto = {}
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    for rep in summary["reps"]:
        for task in rep["tasks"]:
            for arm, rec in (task.get("arms") or {}).items():
                auto[f"{task['task']}|{arm}|{rep['rep']}"] = bool(
                    rec.get("accepted") and _bug_sources(rec))
    agree = total = 0
    for x in compared:
        for c in x["cells"]:
            key = f"{c['task']}|{c['arm']}|{c['rep']}"
            if key not in auto:
                continue
            total += 1
            match = (x["bug_present"] == "yes") == auto[key]
            agree += 1 if match else 0
            if not match:
                disagree.append({"sha256": x["sha256"], **c, "expert": x["bug_present"],
                                 "auto": auto[key]})
    design_loss = {}
    for x in labels:
        if x["design_preserved"] == "no" and x["bug_present"] != "unsure":
            design_loss[x["arm"]] = design_loss.get(x["arm"], 0) + 1
    payload = {"rubric_version": "expert-label-rubric-v2", "labeler": "agent-proxy",
               "seed": SEED, "candidates": len(labels),
               "unsure": len(unsure),
               "unsure_rate": round(len(unsure) / len(labels), 3) if labels else None,
               "agreement": {"compared": total, "agree": agree,
                             "rate": round(agree / total, 3) if total else None},
               "design_loss": design_loss, "labels": labels}
    (OUT / "EXPERT_LABELS.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    # Human queue: all disagreements + all unsure + random 4.
    rng = random.Random(SEED)
    pool = [x for x in labels if x["bug_present"] != "unsure"
            and x["sha256"] not in {d["sha256"] for d in disagree}]
    queue = disagree + [{"sha256": x["sha256"], **x["cells"][0], "expert": x["bug_present"],
                         "auto": auto.get(f"{x['cells'][0]['task']}|{x['cells'][0]['arm']}|{x['cells'][0]['rep']}")}
                        for x in unsure]
    for x in rng.sample(pool, min(4, len(pool))):
        queue.append({"sha256": x["sha256"], **x["cells"][0], "expert": x["bug_present"],
                      "auto": auto.get(f"{x['cells'][0]['task']}|{x['cells'][0]['arm']}|{x['cells'][0]['rep']}")})
    lines = ["# Human review queue (leave blank for the owner)", "",
             f"All expert/automatic disagreements, all `unsure`, plus a random 4 (seed `{SEED}`).",
             "Fill `human_label` (`yes`/`no`/`unsure`) and one reason; do not let an agent fill it.",
             "", "| task | arm | rep | sha256 | agent | auto | human_label | reason |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for q in queue:
        lines.append(f"| {q['task']} | {q['arm']} | {q.get('rep')} | `{q['sha256'][:12]}` | "
                     f"{q['expert']} | {q.get('auto')} | | |")
    (OUT / "HUMAN_REVIEW_QUEUE.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    md = ["# Expert labels v2 (per candidate)", "",
          f"- rubric `expert-label-rubric-v2`; candidates {len(labels)}; "
          f"unsure {len(unsure)} ({payload['unsure_rate']}); "
          f"agreement {agree}/{total}",
          f"- design_loss by arm: {design_loss}", "",
          "| sha256 | task | arm | kind | bug_present | design_preserved | unsure_reason | evidence |",
          "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for x in labels:
        md.append(f"| `{x['sha256'][:12]}` | {x['task']} | {x['arm']} | {x['kind']} | "
                  f"{x['bug_present']} | {x['design_preserved']} | {x['unsure_reason'] or ''} | "
                  f"{x['evidence'][:80]} |")
    (OUT / "EXPERT_LABELS.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print(json.dumps({"candidates": len(labels), "unsure": len(unsure),
                      "unsure_rate": payload["unsure_rate"],
                      "agreement": payload["agreement"],
                      "design_loss": design_loss, "queue": len(queue)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
