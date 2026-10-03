#!/usr/bin/env python3
"""Fresh six-arm DeepSeek repair run; no dependency on saved experiment outputs."""
import argparse
import dataclasses
import hashlib
import json
import os
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
from cir_workflow.env import load_dotenv
from cir_workflow.live import DeepSeekFlashClient, LiveBudget, RecordingLocalProvider, RecordingProvider
from cir_workflow.flash_smoke import RustLiveProvider, ARC
from cir_workflow.channels import AuditedClient
from cir_workflow.audit import AuditLog
from cir_workflow.transport import ModelSpec
from cir_workflow.arms import run_rust_arm, cir_oracle
from cir_workflow.concir_client import ConcirClient
from cir_workflow.revision_workflow import WholeArtifactRevisionWorkflow


def run_repair(config, out):
    load_dotenv(ROOT / ".env")
    if not os.environ.get("DEEPSEEK_API_KEY"): raise RuntimeError("missing DEEPSEEK_API_KEY")
    from cir_workflow.rust_arm import lockbud_path
    if lockbud_path() is None: raise RuntimeError("repair baseline requires Lockbud; see README.md")
    cfg=json.loads(Path(config).read_text())
    out=Path(out).resolve();out.mkdir(parents=True,exist_ok=False)
    budget=LiveBudget(out / "budget.json",max_requests=cfg["max_requests"],max_seconds=cfg["wall_seconds"])
    spec_model=ModelSpec("DeepSeek Flash","deepseek","deepseek-direct","deepseek-flash")
    rows=[]
    meta={"config":cfg,"config_sha256":hashlib.sha256(Path(config).read_bytes()).hexdigest(),
          "cells":rows,"stop":None,"complete":False}
    binary=Path(os.environ["CONCIR_BACKEND"])
    for rep in range(cfg["repetitions"]):
        for task in cfg["tasks"]:
            rt=json.loads((ROOT / "benchmarks/families" / task / "repair_task.json").read_text())
            b=ROOT / "benchmarks"
            contract_path=b / rt["contract"];contract=json.loads(contract_path.read_text())
            requirements=(b / rt["requirements_file"]).read_text();initial=b / rt["input_cir"]
            for arm in cfg["arms"]:
                if budget.exhausted(): meta["stop"]=budget.exhausted();break
                cell=out / task.replace('/','__') / arm / f"rep{rep}";cell.mkdir(parents=True)
                inner=DeepSeekFlashClient(api_key=os.environ["DEEPSEEK_API_KEY"],budget=budget,
                    evidence_dir=cell / "transport",max_tokens=4096,timeout=180)
                llm=AuditedClient(inner,audit=AuditLog(cell / "audit.jsonl"),run_id="reproduce-repair",
                    cell_id=f"{task}/{arm}/{rep}",spec=spec_model,arm=arm,task_id=task,replicate=rep,stage="repair")
                row={"task":task,"arm":arm,"rep":rep}
                try:
                    if arm in ARC:
                        provider=RustLiveProvider(llm,ARC[arm])
                        result=run_rust_arm(provider,arm=arm,task=task,spec=requirements,contract=contract,
                            out_dir=cell / "workflow",k=cfg["rounds"],initial_source=(b / rt["input_rust"]).read_text(),
                            tool_timeout_s=8,run_miri_many_seeds=False)
                        row["result"]=result.as_dict();row["accepted"]=result.accepted
                    else:
                        client=ConcirClient(binary,workdir=cell / "calls")
                        phases=[("local",cfg["rounds"])] if arm=="A3_local" else \
                               [("whole",cfg["rounds"])] if arm=="A3_whole" else \
                               [("local",cfg["local_phase_rounds"]),("whole",cfg["rounds"]-cfg["local_phase_rounds"])]
                        current=initial;row["phases"]=[]
                        for index,(fmt,limit) in enumerate(phases):
                            # The initial artifact is inspected as version one; +1 permits limit LLM proposals.
                            provider=RecordingLocalProvider(llm,current.read_text()) if fmt=="local" else RecordingProvider(llm)
                            wf=WholeArtifactRevisionWorkflow(client,provider,out_dir=cell / f"phase-{index}",
                                max_rounds=limit+1,reply_format=fmt)
                            result=wf.run(requirements,contract,task_id=task,initial_program=current)
                            row["phases"].append(result.as_dict())
                            for version in reversed(result.versions):
                                if version.artifact_path: current=Path(version.artifact_path);break
                            if result.accepted: break
                        row.update(accepted=result.accepted,final_cir=str(current),oracle=cir_oracle(client,current,contract_path))
                except Exception as exc:
                    row.update(accepted=False,error=f"{type(exc).__name__}: {exc}")
                    meta["stop"]=row["error"]
                rows.append(row)
                (out / "SUMMARY.json").write_text(json.dumps(meta,indent=2)+"\n")
                if meta["stop"]: break
            if meta["stop"]: break
        if meta["stop"]: break
    meta.update(complete=not meta["stop"] and len(rows)==len(cfg["tasks"])*len(cfg["arms"])*cfg["repetitions"],
                requests_used=budget.requests_used)
    (out / "SUMMARY.json").write_text(json.dumps(meta,indent=2)+"\n")
    print(json.dumps({"cells":len(rows),"complete":meta["complete"],"stop":meta["stop"]}))
    if not meta["complete"]: raise RuntimeError("repair batch incomplete; inspect SUMMARY.json")


if __name__ == "__main__":
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--config',default=str(ROOT / 'configs/repair.json'));p.add_argument('--out',required=True)
    a=p.parse_args();run_repair(a.config,a.out)
