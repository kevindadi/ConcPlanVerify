import sys,json,copy,argparse,io,contextlib,hashlib
from pathlib import Path
sys.dont_write_bytecode=True
repo=Path('/Users/kevin/local-repos/ConcIR');out=Path('/private/tmp/concir-lifecycle-review/probes');out.mkdir(exist_ok=True)
sys.path.insert(0,str(repo/'scripts'));import run_pilot as r;import pilot_analyze as a;import pilot_audit as audit
m=json.loads((repo/'experiments/pilot-v2/manifest.json').read_text());case=copy.deepcopy(m['cases'][0]);case['repeat_policy']={'main':1,'tight':0}
for k in ['model','contract']:case[k]=str(repo/'experiments/pilot-v2'/case[k])
mp=out/'mini.json';mp.write_text(json.dumps({'cases':[case],'search_configs':m['search_configs']}))
args=argparse.Namespace(out=str(out),manifest=str(mp),bin=str(repo/'target/release/concir-backend'),tool=str(repo/'target/release/examples/pilot_tool'),suite=['pilot'],search_timeout=30.,replay_timeout=30.,total_budget=60.,resume=False,batch_tag='missing',allow_overrun=False)
with contextlib.redirect_stdout(io.StringIO()):assert r.do_run(args)==0
bd=out/'results/batches/missing';rows=[json.loads(x) for x in (bd/'valid_index.jsonl').read_text().splitlines()];b=next(x for x in rows if x['strategy']=='b');Path(b['artifact_path']).unlink()
real=r.run_process
def failing(cmd,timeout_s,cwd=r.REPO):
 if len(cmd)>1 and cmd[1]=='repair':return dict(cmd=cmd,exit_code=2,stdout='',stderr='injected failed retry',wall_ms=1,timeout_s=timeout_s,timed_out=False,spawn_error=None)
 return real(cmd,timeout_s,cwd)
r.run_process=failing;args.resume=True
with contextlib.redirect_stdout(io.StringIO()):rc=r.do_run(args)
r.run_process=real
rows2=[json.loads(x) for x in (bd/'valid_index.jsonl').read_text().splitlines()];b2=next(x for x in rows2 if x['strategy']=='b');attempts=[json.loads(x) for x in (bd/'attempts.jsonl').read_text().splitlines()]
with contextlib.redirect_stdout(io.StringIO()):src=a.summarize_batch(bd,out)
findings={'invalid_old_evidence':{'run_exit':rc,'new_attempt_status':attempts[-1]['evidence_status'],'indexed_status':b2['evidence_status'],'indexed_classification':b2['final_classification'],'artifact_exists':Path(b2['artifact_path']).exists(),'summarize_exit':src,'audit_issues':audit.audit(out)['issues']}}
# Recovery is already exercised by actual lifecycle CLI; inspect its B/C wall bases.
life=next(Path('/private/tmp/concir-lifecycle-review').glob('lifecycle-*'))
rec=[json.loads(x) for x in (life/'results/batches/b5/valid_index.jsonl').read_text().splitlines()]
findings['recovery_wall_basis']=[{k:x.get(k) for k in ['strategy','wall_ms','search_wall_ms','replay_wall_ms','wall_ms_basis']} for x in rec]
# The same actual complete B paired with a C search timeout must retain the paired outcome comparison.
stat=out/'stats';stat.mkdir(exist_ok=True)
source=[json.loads(x) for x in (repo/'experiments/pilot-v2-lifecycle/results/batches/smoke-lifecycle/valid_index.jsonl').read_text().splitlines()]
base=[x for x in source if x['case']=='single_cycle' and x['strategy'] in ['b','c']]
for x in base:
 if x['strategy']=='c':x.update(evidence_status='search_timeout',final_classification='search_timeout',raw_outcome=None,replay_ok=None,replay_exit=None)
(stat/'valid_index.jsonl').write_text(''.join(json.dumps(x)+'\n' for x in base));(stat/'plan.json').write_text(json.dumps([{'run_key':x['run_key']} for x in base]));(stat/'batch.json').write_text('{}')
with contextlib.redirect_stdout(io.StringIO()):rc=a.summarize_batch(stat,out)
s=json.loads((stat/'summary.json').read_text());findings['success_timeout_pair']={'input':[(x['strategy'],x['final_classification']) for x in base],'summary_exit':rc,'pairs':s['paired']['pairs'],'success_differs':s['paired']['success_differs']}
Path('/private/tmp/concir-lifecycle-review/probes.json').write_text(json.dumps(findings,indent=2)+'\n');print(json.dumps(findings,indent=2))
