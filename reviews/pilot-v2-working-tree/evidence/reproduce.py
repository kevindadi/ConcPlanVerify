import sys,json,copy,hashlib,subprocess,argparse,time,contextlib,io
from pathlib import Path
sys.dont_write_bytecode=True
root=Path('/private/tmp/concir-pilot-v2-audit')
sys.path.insert(0,str(root/'workspace/scripts'))
import run_pilot as r
import pilot_analyze as a
src=Path('/Users/kevin/local-repos/ConcIR')
out=root/'workspace/repro'
mp=root/'workspace/experiments/pilot-v2/mini.json'
m=json.loads((src/'experiments/pilot-v2/manifest.json').read_text())
c=next(c for c in m['cases'] if c['case']=='p1_same')
c['model']=str(src/'experiments/pilot-v2'/c['model']);c['contract']=str(src/'experiments/pilot-v2'/c['contract'])
c['repeat_policy']={'main':1,'tight':0}
m['cases']=[c];m['smoke_cases']=[];m['matrix_cases']=[]
mp.write_text(json.dumps(m))
args=argparse.Namespace(out=str(out),manifest=str(mp),bin=str(src/'target/release/concir-backend'),tool=str(src/'target/release/examples/pilot_tool'),suite=['pilot'],search_timeout=30.,replay_timeout=30.,total_budget=60.,resume=False,batch_tag='restart',allow_overrun=False)
# Real runner twice, unchanged batch tag/config, second search faults and produces nothing.
with contextlib.redirect_stdout(io.StringIO()): r.do_run(args)
bdir=out/'results/batches/restart'
readidx=lambda:[json.loads(l) for l in (bdir/'valid_index.jsonl').read_text().splitlines()]
old=readidx();snap={x['artifact_path']:Path(x['artifact_path']).read_bytes() for x in old}
real=r.run_process
def fail(cmd,timeout_s,cwd=r.REPO):
    if len(cmd)>1 and cmd[1]=='repair':
        return dict(cmd=cmd,exit_code=2,stdout='',stderr='injected failure without artifact',wall_ms=1,timeout_s=timeout_s,timed_out=False,spawn_error=None)
    return real(cmd,timeout_s,cwd)
r.run_process=fail
with contextlib.redirect_stdout(io.StringIO()): r.do_run(args)
r.run_process=real
now=readidx();ats=[json.loads(l) for l in (bdir/'attempts.jsonl').read_text().splitlines()]
result={'same_batch_failure':{'attempt_rows':len(ats),'unique_attempt_dirs':len({x['attempt_dir'] for x in ats}), 'current_attempt_statuses':[x['evidence_status'] for x in ats[-3:]], 'index_statuses':[x['final_classification'] for x in now], 'indexed_exit_vs_disk':[[x['exit_code'],(Path(x['attempt_dir'])/'search.exit').read_text()] for x in now]}}
# Changing config under same batch tag overwrites old artifact and leaves old records.
m['search_configs']['main']['candidate_budget']+=1
mp.write_text(json.dumps(m))
with contextlib.redirect_stdout(io.StringIO()): r.do_run(args);a.summarize_batch(bdir,out)
now=readidx();s=json.loads((bdir/'summary.json').read_text())
result['same_batch_changed_config']={'index_records':len(now),'plan_records':len(json.loads((bdir/'plan.json').read_text())),'unique_artifact_paths':len({x['artifact_path'] for x in now}),'bad_artifact_hash_records':sum(hashlib.sha256(Path(x['artifact_path']).read_bytes()).hexdigest()!=x['artifact_sha256'] for x in now),'summary_records':s['records']}
# Simulate legitimate completed search awaiting replay by changing only pending status.
pending=copy.deepcopy(next(x for x in now if x['config']['candidate_budget']==m['search_configs']['main']['candidate_budget'] and x['strategy']=='b'))
pending.update(evidence_status='replay_pending',final_classification='replay_pending',replay_ok=None,replay_exit=None)
(bdir/'valid_index.jsonl').write_text(json.dumps(pending)+'\n')
args.resume=True
with contextlib.redirect_stdout(io.StringIO()): r.do_run(args)
new=next(x for x in readidx() if x['strategy']=='b')
result['replay_pending_recovery']={'status':new['evidence_status'],'classification':new['final_classification'],'replay_ok':new.get('replay_ok'),'missing_fields':[k for k in ['artifact_path','artifact_sha256','exit_code','search_wall_ms','verification_calls','states_explored','patch_len','root_outcome','wall_ms'] if k not in new]}
(root/'reproductions.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
