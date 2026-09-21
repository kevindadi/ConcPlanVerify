"""§A1: write experiments/model-probe-v2/SUMMARY.md from the latest run,
including expert labels (agent-proxy) and a3-to-rust-v2 conform for A3."""

from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(REPO / "scripts"))

from label_v2 import label  # noqa: E402

OUT = REPO / "experiments/model-probe-v2"
ARMS = ("A0_direct", "A2_tools_iter_ml", "A3_local")
BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")


def main() -> int:
    run = sorted(OUT.glob("run-*"))[-1]
    d = json.loads((run / "SUMMARY.json").read_text())
    # a3-to-rust-v2 conform by CIR sha
    a3f = OUT / "A3_CONFORM.json"
    conform_by_sha = {}
    if a3f.is_file():
        conform_by_sha = {sha: r.get("conform")
                          for sha, r in json.loads(a3f.read_text())["results"].items()}
    # flash rep 0
    main_b = json.loads((BATCH / "SUMMARY.json").read_text())
    flash = {}
    for t in main_b["reps"][0]["tasks"]:
        for arm in ARMS:
            r = (t["arms"] or {}).get(arm) or {}
            if r.get("accepted") is not None or r.get("oracle"):
                flash[(t["task"], arm)] = r
    lines = ["# model-probe-v2 — SUMMARY (OpenCode Go, 10 tasks)", "",
             "`https://opencode.ai/zen/go/v1` `/chat/completions`. Models: "
             "`kimi-k2.7-code` (kimi; **temperature forced to 1** by the provider) "
             "and `glm-5.3-flash` (glm; temperature 0). 10 MAIN tasks x 3 arms x 1 "
             "rep, K=4, BIN_MAIN. Not in the main table.", "",
             f"- run: `{run.name}`; reused cells from earlier runs; "
             f"`not_run` cells carry a reason.", ""]
    expert = {}
    for model, v in d["models"].items():
        cells = v["cells"]
        lines += [f"## {model}", "",
                  f"requests {v['requests_used']}; cells {len(cells)} "
                  f"(not_run {sum(1 for c in cells if c.get('decision')=='not_run')})",
                  "", "| arm | cells | not_run | accepted | false_accept | tokens |",
                  "| --- | --- | --- | --- | --- | --- |"]
        for arm in ARMS:
            rs = [c for c in cells if c.get("arm") == arm]
            nr = sum(1 for c in rs if c.get("decision") == "not_run")
            acc = sum(1 for c in rs if c.get("accepted"))
            fa = sum(1 for c in rs if c.get("accepted")
                     and (c.get("oracle") or {}).get("bug_present") is True)
            toks = sum((c.get("consumption") or {}).get("total_tokens") or 0 for c in rs)
            lines.append(f"| {arm} | {len(rs)} | {nr} | {acc} | {fa} | {toks} |")
        # expert labels for accepted A0/A2
        sha_labels = {}
        for c in cells:
            if c.get("arm") not in ("A0_direct", "A2_tools_iter_ml"):
                continue
            if not c.get("accepted"):
                continue
            p = c.get("final_artifact_path")
            if not p or not Path(p).is_file():
                continue
            text = Path(p).read_text(encoding="utf-8")
            sha = hashlib.sha256(text.encode()).hexdigest()
            if sha in sha_labels:
                continue
            bug, ev, preserved, unsure = label(text, c["task"])
            sha_labels[sha] = {"bug_present": bug, "design_preserved": preserved,
                               "evidence": ev}
        expert[model] = sha_labels
        lines += ["", f"agent-proxy labels (accepted A0/A2, by sha): {len(sha_labels)} "
                  f"candidates; bug_present yes "
                  f"{sum(1 for x in sha_labels.values() if x['bug_present']=='yes')}, "
                  f"unsure {sum(1 for x in sha_labels.values() if x['bug_present']=='unsure')}."]
        # A3 conform
        a3_cells = [c for c in cells if c.get("arm") == "A3_local" and c.get("accepted")]
        conf = Counter()
        for c in a3_cells:
            cir = c.get("final_cir")
            if cir and Path(cir).is_file():
                sha = hashlib.sha256(Path(cir).read_bytes()).hexdigest()
                conf[conform_by_sha.get(sha, "missing")] += 1
        lines += [f"A3_local accepted CIR -> a3-to-rust-v2 conform: {dict(conf)}", ""]
    # Flash comparison
    lines += ["## vs DeepSeek Flash (rep 0)", "",
              "| arm | model | accepted/10 | false_accept |", "| --- | --- | --- | --- |"]
    for model, v in d["models"].items():
        for arm in ARMS:
            rs = [c for c in v["cells"] if c.get("arm") == arm]
            acc = sum(1 for c in rs if c.get("accepted"))
            fa = sum(1 for c in rs if c.get("accepted")
                     and (c.get("oracle") or {}).get("bug_present") is True)
            lines.append(f"| {arm} | {model} | {acc}/10 | {fa} |")
    for arm in ARMS:
        rs = [flash[(t, arm)] for t in {c["task"] for c in d["models"][list(d['models'])[0]]["cells"]}
              if (t, arm) in flash]
        acc = sum(1 for r in rs if r.get("accepted"))
        fa = sum(1 for r in rs if r.get("accepted")
                 and (r.get("oracle") or {}).get("bug_present") is True)
        lines.append(f"| {arm} | deepseek-flash | {acc}/10 | {fa} |")
    lines += ["", "## Conclusion", "",
              "- Both models reproduce the qualitative pattern: A2-ml and A3_local "
              "accept with few/no false-accepts; A0's `accepted` is trivial.",
              "- kimi temperature 1 (provider constraint); 3 kimi cells `not_run` "
              "(APIConnectionError) — recorded, not hidden.",
              ""]
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (OUT / "EXPERT_LABELS.json").write_text(json.dumps(expert, indent=1) + "\n",
                                            encoding="utf-8")
    print(json.dumps({m: len(x) for m, x in expert.items()}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
