"""Emit a denser goal-constrained deadlock CIR (5 workers, more clutter)."""

from __future__ import annotations

import json
from pathlib import Path


def _lock_arm(start_sid: int, order: list[str], write_val: str, ret_sid: str) -> list[dict]:
    body = []
    sid = start_sid
    seq = [(n, "lock") for n in order] + [("result", "write", write_val)] + [
        (n, "drop") for n in reversed(order)
    ]
    for i, item in enumerate(seq):
        nxt = f"s{sid + 1}" if i < len(seq) - 1 else ret_sid
        if item[1] == "write":
            op = ["res_op", item[0], "write", item[2]]
        else:
            op = ["res_op", item[0], item[1]]
        body.append({"sid": f"s{sid}", "op": op, "transfer": ["next", nxt]})
        sid += 1
    return body, sid


def build(*, buggy: bool) -> dict:
    workers = [f"w{i}" for i in range(1, 5)]  # 4 workers: fits under max_states=100k
    main_body = []
    sid = 1
    for w in workers:
        main_body.append({"sid": f"s{sid}", "op": ["spawn", w], "transfer": ["next", f"s{sid+1}"]})
        sid += 1
    for w in workers:
        main_body.append({"sid": f"s{sid}", "op": ["join", w], "transfer": ["next", f"s{sid+1}"]})
        sid += 1
    main_body.append({"sid": f"s{sid}", "op": "return", "transfer": "return"})

    functions = [{"name": "main", "kind": "normal", "body": main_body}]
    for i, w in enumerate(workers, 1):
        if i != 3:
            true_arm, next_sid = _lock_arm(3, ["m1", "m2"], str(i), "s13")
            false_arm, _ = _lock_arm(next_sid, ["m1", "m2"], str(i + 10), "s13")
            body = [
                {"sid": "s1", "op": ["res_op", "flag", "write", str(i)], "transfer": ["next", "s2"]},
                {"sid": "s2", "op": ["res_op", "flag", "read"],
                 "transfer": ["branch", f"flag == {i}", "s3", f"s{next_sid}"]},
                *true_arm,
                *false_arm,
                {"sid": "s13", "op": "return", "transfer": "return"},
            ]
            functions.append({"name": w, "kind": "closure", "body": body})
        else:
            order_b = ["m2", "m1"] if buggy else ["m1", "m2"]
            false_arm, _ = _lock_arm(7, order_b, "99", "s12")
            body = [
                {"sid": "s1", "op": ["res_op", "flag", "write", "3"], "transfer": ["next", "s2"]},
                {"sid": "s2", "op": ["res_op", "flag", "read"],
                 "transfer": ["branch", "flag == 3", "s3", "s7"]},
                {"sid": "s3", "op": ["res_op", "m1", "lock"], "transfer": ["next", "s4"]},
                {"sid": "s4", "op": ["res_op", "result", "write", "3"], "transfer": ["next", "s5"]},
                {"sid": "s5", "op": ["res_op", "m1", "drop"], "transfer": ["next", "s6"]},
                {"sid": "s6", "op": "return", "transfer": "return"},
                *false_arm,
                {"sid": "s12", "op": "return", "transfer": "return"},
            ]
            functions.append({"name": "w3", "kind": "closure", "body": body})

    return {
        "program": "goal_constrained_deadlock_dense",
        "resources": [
            {"name": "m1", "kind": "sync", "type": "Mutex", "mode": "Sync"},
            {"name": "m2", "kind": "sync", "type": "Mutex", "mode": "Sync"},
            {"name": "flag", "kind": "var", "type": "Var", "base": "Int", "init": 0},
            {"name": "result", "kind": "var", "type": "Var", "base": "Int", "init": 0},
        ],
        "protection": [{"var": "result", "lock": "m1"}],
        "functions": functions,
        "fn_summaries": [],
        "entry": "main",
        "goals": [{
            "id": "g_result_special",
            "desc": "Business payload: some schedule must observe result == 99",
            "marking": {},
            "variables": {"result": 99},
        }],
    }


root = Path("benchmarks/cir/goal_constrained_deadlock_dense")
root.mkdir(parents=True, exist_ok=True)
for name, buggy in [("buggy.json", True), ("fixed.json", False)]:
    (root / name).write_text(
        json.dumps(build(buggy=buggy), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print("wrote", root / name)
