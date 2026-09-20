"""Offline reclassification of A0/A1 replies in the v3 batch (§1.3).

No requests are sent: each round's raw reply is recovered from the LLM evidence
(mapped by request_id) or from `round-N/candidate.rs`, then replayed through the
new three-way classifier and the claims_no_issue adjudication.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.arms import classify_reply  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-smoke-v3/"
                "run-20260919T065313-94177-958361")
TASKS = ("lock-order/partial_deadlock_bystander", "lock-order/cross_module_cycle",
         "lock-order/cycle_3lock", "structure/nested_scope_lock_order",
         "condvar/notify_one_multi_waiter_wrong_pick",
         "channel/bounded_backpressure_lock_held",
         "channel/send_while_holding_mutex",
         "semaphore/acquire_twice_no_release")


def _evidence_by_request_id() -> dict[str, str]:
    out = {}
    for path in sorted((BATCH / "llm").glob("llm-*.json")):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        rid = data.get("request_id")
        if rid:
            out[rid] = data.get("content", "")
    return out


def _round_text(arm_dir: Path, round_no: int, request_id: str | None,
                evidence: dict[str, str]) -> tuple[str, str]:
    if request_id and request_id in evidence:
        return evidence[request_id], "llm-evidence"
    candidate = arm_dir / f"round-{round_no}" / "candidate.rs"
    if candidate.is_file():
        return candidate.read_text(encoding="utf-8"), "candidate.rs"
    return "", "unavailable"


def main() -> int:
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    evidence = _evidence_by_request_id()
    rows = []
    final: dict[tuple[str, str], tuple[bool, int | None]] = {}
    order: list[tuple[str, str]] = []
    for task in summary["tasks"]:
        tid = task["task"]
        if tid not in TASKS:
            continue
        for arm in ("A0_direct", "A1_self_iter"):
            rec = task["arms"].get(arm)
            if not rec:
                continue
            runs = sorted((BATCH / tid.replace("/", "__") / arm).glob("run-*"))
            if not runs:
                continue
            arm_dir = runs[-1]
            build_ok_rounds = {c["round"] for c in
                               (rec.get("notes", {}).get("round_columns") or [])
                               if c.get("build_ok")}
            calls = rec.get("llm_calls", [])
            last_build_ok = None
            new_accepted = False
            new_accepted_round = None
            for i, rnd in enumerate(rec.get("rounds", [])):
                request_id = calls[i].get("request_id") if i < len(calls) else None
                text, source = _round_text(arm_dir, rnd["round"], request_id, evidence)
                cls = classify_reply(text)
                if cls["kind"] == "program":
                    new_decision = "continue" if arm == "A1_self_iter" else rnd["decision"]
                    if rnd["round"] in build_ok_rounds:
                        last_build_ok = rnd["round"]
                        if arm == "A0_direct" and not new_accepted:
                            new_accepted, new_accepted_round = True, rnd["round"]
                elif cls["kind"] == "claims_no_issue":
                    if arm == "A0_direct":
                        new_decision = "claims_no_issue"
                        if not new_accepted:
                            new_accepted, new_accepted_round = True, rnd["round"]
                    elif last_build_ok is not None:
                        new_decision = "claims_no_issue"
                        if not new_accepted:
                            new_accepted, new_accepted_round = True, last_build_ok
                    else:
                        new_decision = "claims_no_issue_unbuilt"
                else:
                    new_decision = "format_error"
                rows.append({
                    "task": tid, "arm": arm, "round": rnd["round"],
                    "reply_source": source, "kind": cls["kind"],
                    "matched": cls.get("matched", []),
                    "old_decision": rnd["decision"], "new_decision": new_decision,
                    "old_accepted": rec.get("accepted"),
                    "new_accepted": new_accepted,
                    "new_accepted_round": new_accepted_round,
                })
                final[(tid, arm)] = (new_accepted, new_accepted_round)
                if (tid, arm) not in order:
                    order.append((tid, arm))
    lines = ["# flash-repair-smoke-v3 — A0/A1 reply reclassification (§1.3)", "",
             f"- batch: `{BATCH.name}` (offline, no requests)",
             "- classifier: three-way (`program` / `claims_no_issue` / `other`);",
             "  see `python/cir_workflow/arms.py:classify_reply` and the word list",
             "  in `flash-repair-main-v1/PROTOCOL.md`.", "",
             "| task | arm | round | reply.kind | old | new | new accepted |",
             "| --- | --- | --- | --- | --- | --- | --- |"]
    for r in rows:
        acc = str(r["new_accepted"])
        if r["new_accepted_round"]:
            acc += f" (r{r['new_accepted_round']})"
        lines.append(f"| {r['task']} | {r['arm']} | {r['round']} | {r['kind']} | "
                     f"{r['old_decision']} | {r['new_decision']} | {acc} |")
    lines += ["", "## Per-cell outcome change", "",
              "| task | arm | old accepted | new accepted | new accepted round |",
              "| --- | --- | --- | --- | --- |"]
    old_accepted = {(r["task"], r["arm"]): r["old_accepted"] for r in rows}
    for key in order:
        acc, rnd = final[key]
        lines.append(f"| {key[0]} | {key[1]} | {old_accepted.get(key)} | "
                     f"{acc} | {rnd} |")
    lines += ["", "## Notes", "",
              "- `notify_one/A0`: `NO_ISSUES` becomes `claims_no_issue` (accept the buggy",
              "  input, `false_accept=True`, `bug_present` by construction).",
              "- `cross_module_cycle/A1`: rounds 2–4 are fence-free prose ('same order')",
              "  → `claims_no_issue`; round 1 built, so the new rule accepts the round-1",
              "  candidate instead of burning the remaining rounds.",
              "- `send_while/A1`: round 3 `NO_ISSUES`; round 1 built, so it is accepted on",
              "  the round-1 candidate (the old rule accepted the r3 non-built state).",
              "- `format_error` rounds are what the new one-retry-per-reply rule targets.",
              "  Limitation: the new harness issues the retry *within* the same round, so for",
              "  arms with an `other` round before a claim (e.g. `notify_one/A1` r2) the",
              "  replayed round indices after that point are indicative, not exact.",
              ""]
    out = REPO / "experiments/flash-repair-smoke-v3/REPLY_RECLASS.md"
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {out} ({len(rows)} rounds)")
    for r in rows:
        if r["kind"] == "claims_no_issue" or r["old_decision"] != r["new_decision"]:
            print(r["task"], r["arm"], "r", r["round"], r["kind"],
                  r["old_decision"], "->", r["new_decision"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
