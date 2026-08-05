"""Compare repair_local vs repair_cvn (full) A/B results."""

import json
from pathlib import Path

RESULTS = Path(__file__).resolve().parents[1] / "results"


def load(name):
    return {
        r["case_id"]: r
        for r in json.loads((RESULTS / name).read_text(encoding="utf-8"))["records"]
    }


local = load("repair_local.json")
full = load("repair_full_v2.json")

header = (
    f"{'case':28s} {'local':>6s} {'rounds':>6s} {'tokens':>8s} {'slice':>12s} "
    f"{'fb':>3s} | {'full':>5s} {'rounds':>6s} {'tokens':>8s}"
)
print(header)
print("-" * len(header))
for case_id in local:
    l = local[case_id]
    f = full.get(case_id, {})
    slice_view = f"{len(l.get('initial_slice') or [])}/{l.get('total_functions')}"
    if l.get("slice_expanded"):
        slice_view += "+"
    print(
        f"{case_id:28s} {str(l.get('success')):>6s} {l.get('repair_rounds', -1):>6d} "
        f"{l.get('total_tokens', 0):>8d} {slice_view:>12s} "
        f"{('Y' if l.get('fell_back') else '-'):>3s} | "
        f"{str(f.get('success')):>5s} {f.get('repair_rounds', -1):>6d} "
        f"{f.get('total_tokens', 0):>8d}"
    )

l_ok = sum(1 for r in local.values() if r.get("success"))
f_ok = sum(1 for r in full.values() if r.get("success"))
l_tok = sum(r.get("total_tokens", 0) for r in local.values())
f_tok = sum(r.get("total_tokens", 0) for r in full.values())
fell = sum(1 for r in local.values() if r.get("fell_back"))
print(
    f"\nlocal: {l_ok}/{len(local)} ok, {l_tok:,} tokens total, {fell} fell back"
    f" | full: {f_ok}/{len(full)} ok, {f_tok:,} tokens total"
)

# Drift check: did the resource type mix change between input and fixed CIR?
print("\nresource-type drift (input -> fixed):")
for name, data in (("local", local), ("full", full)):
    for case_id, r in data.items():
        if not r.get("success"):
            continue
        before = (r.get("cir_metrics_input") or {}).get("resource_by_type")
        after = (r.get("cir_metrics_fixed") or {}).get("resource_by_type")
        if before and after and before != after:
            print(f"  [{name}] {case_id}: {before} -> {after}")
print("  (no output above = no drift)")
