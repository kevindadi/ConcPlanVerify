"""Quick correctness summary for an offline analyze result file."""

import json
import sys

path = sys.argv[1] if len(sys.argv) > 1 else "results/offline_v3.json"
data = json.load(open(path, encoding="utf-8"))
runs = data["records"] if isinstance(data, dict) and "records" in data else data
bad = 0
for r in runs:
    score = r.get("score") or {}
    ok = score.get("correct")
    if not ok:
        bad += 1
    kinds = score.get("detected_kinds")
    status = score.get("status") or (r.get("rust_cli") or {}).get("status")
    print(f"{r['case_id']:32s} correct={ok!s:5s} status={status} kinds={kinds}")
print("total incorrect:", bad)
