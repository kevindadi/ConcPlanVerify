import json
from pathlib import Path


def load(path):
    p = Path(path)
    if not p.exists():
        return None
    return json.load(open(p, encoding="utf-8"))


deep = load("results/deep_repair.json")
if deep:
    print("== deep_lock_chain_4x3 repair A/B ==")
    for r in deep["records"]:
        rounds = r.get("repair_rounds")
        tok = (r.get("total_input_tokens") or 0) + (r.get("total_output_tokens") or 0)
        print(
            f"  {r['method']}: success={r.get('success')} rounds={rounds} "
            f"tokens={tok} error={r.get('error')}"
        )
        for rd in r.get("rounds") or []:
            v = rd.get("verification") or {}
            cm = rd.get("cvn_metrics") or {}
            print(
                f"    round {rd['round']}: accepted={rd.get('accepted')} "
                f"status={v.get('status')} kinds={cm.get('bug_kinds')} "
                f"parse_err={bool(rd.get('parse_error'))}"
            )

judge = load("results/llm_judge.json")
if judge:
    print("\n== llm_judge (16 cases) ==")
    det = fp = miss = 0
    n_bug = n_safe = 0
    for r in judge["records"]:
        sc = r.get("score") or {}
        if not sc:
            print(f"  {r['case_id']}: ERROR {r.get('error')}")
            continue
        exp, act = sc.get("expected"), sc.get("claimed")
        j = r.get("judge") or {}
        mark = ""
        if sc.get("false_positive"):
            mark = " FALSE-POSITIVE"
            fp += 1
        if sc.get("missed"):
            mark = " MISSED"
            miss += 1
        if sc.get("detected"):
            det += 1
        if exp == "safe":
            n_safe += 1
        else:
            n_bug += 1
        print(
            f"  {r['case_id']}: gold={exp} claimed={act} "
            f"kind={j.get('bug_kind')} suspects={j.get('suspect_functions')}{mark}"
        )
    print(f"  -> detected {det}/{n_bug} defects, false positives {fp}/{n_safe} safe cases")

cg = load("results/codegen.json")
if cg:
    print("\n== codegen (verified CIR -> Rust) ==")
    ok = n = 0
    for r in cg["records"]:
        if r.get("skipped"):
            print(f"  {r['case_id']}: skipped ({r['skipped']})")
            continue
        n += 1
        cmx = r.get("code_metrics") or {}
        tok = (r.get("total_input_tokens") or 0) + (r.get("total_output_tokens") or 0)
        if r.get("success"):
            ok += 1
        print(
            f"  {r['case_id']}: success={r.get('success')} rounds={r.get('codegen_rounds')} "
            f"loc={cmx.get('loc')} spawns={cmx.get('thread_spawns')} tokens={tok}"
        )
    print(f"  -> {ok}/{n} cargo check passed")
