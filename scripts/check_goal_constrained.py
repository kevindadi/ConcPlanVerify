import json
import subprocess
from pathlib import Path

bin_path = Path("target/release/cir2cvn.exe")
if not bin_path.exists():
    bin_path = Path("target/release/cir2cvn")


def analyze(path: str) -> dict:
    out = subprocess.run(
        [str(bin_path), "--analyze", path],
        capture_output=True, text=True, encoding="utf-8",
    )
    return json.loads(out.stdout)


for path in [
    "benchmarks/cir/goal_constrained_deadlock/buggy.json",
    "benchmarks/cir/goal_constrained_deadlock/fixed.json",
]:
    p = analyze(path)
    kinds = [
        next(iter(b["kind"])) if isinstance(b.get("kind"), dict) else b.get("kind")
        for b in (p.get("bugs") or [])
    ]
    print(
        path.split("/")[-1],
        "status=", p.get("status"),
        "states=", p.get("state_count"),
        "kinds=", sorted(set(kinds)),
        "unmet=", len(p.get("unmet_goals") or []),
        "warnings=", len(p.get("goal_warnings") or []),
    )

# Prove normalize-style rewrite that drops write-99 fails goals.
broken = json.loads(Path("benchmarks/cir/goal_constrained_deadlock/fixed.json").read_text(encoding="utf-8"))
for fn in broken["functions"]:
    if fn["name"] != "w3":
        continue
    for stmt in fn["body"]:
        op = stmt.get("op")
        if isinstance(op, list) and op[:3] == ["res_op", "result", "write"] and op[3] == "99":
            op[3] = "3"  # normalize: else arm writes same as if arm
out = subprocess.run(
    [str(bin_path), "--analyze", "-"],
    input=json.dumps(broken), capture_output=True, text=True, encoding="utf-8",
)
p = json.loads(out.stdout)
print(
    "normalize_drop_99:",
    "status=", p.get("status"),
    "unmet=", [g.get("id") for g in (p.get("unmet_goals") or [])],
)
