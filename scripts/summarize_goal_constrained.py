import json
from pathlib import Path


def show(path: str) -> None:
    p = Path(path)
    if not p.exists():
        print(f"(missing {path})")
        return
    d = json.load(open(p, encoding="utf-8"))
    print(f"== {path} ==")
    for r in d["records"]:
        probe = r.get("fixed_goal_probe") or {}
        tok = (r.get("total_input_tokens") or 0) + (r.get("total_output_tokens") or 0)
        print(
            f"  {r['case_id']:36} {r['method']:20} "
            f"ok={r.get('success')} rounds={r.get('repair_rounds')} "
            f"has99={probe.get('has_result_99')} tok={tok} err={r.get('error')}"
        )
        for rd in r.get("rounds") or []:
            if rd.get("accepted"):
                continue
            v = rd.get("verification") or {}
            cm = rd.get("cvn_metrics") or {}
            print(
                f"    fail r{rd['round']} status={v.get('status')} "
                f"unmet={cm.get('unmet_goal_count')} kinds={cm.get('bug_kinds')} "
                f"parse={bool(rd.get('parse_error'))}"
            )


show("results/goal_constrained_repair_ab.json")
show("results/goal_constrained_flash.json")
