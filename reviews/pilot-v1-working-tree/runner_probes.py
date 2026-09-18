import sys,importlib.util,json,copy,pathlib,types,time,contextlib,io
sys.dont_write_bytecode=True
p=pathlib.Path('/private/tmp/concir-pilot-audit');root=pathlib.Path('/Users/kevin/local-repos/ConcIR')
spec=importlib.util.spec_from_file_location('pilot',root/'scripts/run_pilot.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
manifest=json.loads(m.MANIFEST.read_text());ce=next(c for c in manifest['cases'] if c['case']=='p1_same');cfg=manifest['search_configs']['main'];binary=m.DEFAULT_BIN;bsha=m.sha256_file(binary);sid=m.git_info()['source_id'];out=p/'runner_cases';out.mkdir(exist_ok=True);res=out/'results.jsonl';summary={}
rec,status=m.run_one(ce,'main',cfg,'b',1,binary,bsha,sid,out,5,None,res);summary['clean']={k:rec[k] for k in ['final_classification','replay_ok','exit_code']}
original=m.run_process
def fail_repair(cmd,timeout_s,cwd=m.REPO):
 if 'repair' in list(map(str,cmd)):
  return {'cmd':list(map(str,cmd)),'exit_code':2,'stdout':'','stderr':'injected current-run input failure','wall_ms':1,'timeout_s':timeout_s,'timed_out':False}
 return original(cmd,timeout_s,cwd)
m.run_process=fail_repair
newcfg={**cfg,'candidate_budget':0};stale,_=m.run_one(ce,'main',newcfg,'b',1,binary,bsha,sid,out,5,None,res)
summary['stale_failure']={k:stale[k] for k in ['final_classification','replay_ok','exit_code','config','proposals','run_fingerprint']};summary['stale_failure']['artifact_candidate_budget']=json.loads(pathlib.Path(stale['artifact_path']).read_text())['effective_config']['candidate_budget'];m.run_process=original
with contextlib.redirect_stdout(io.StringIO()):m.do_summarize(types.SimpleNamespace(out=str(out)))
s=json.loads((out/'summary.json').read_text());summary['summary_after_same_key_rerun']={'records':s['records'],'by_suite':s['by_suite'],'nondeterministic':s['nondeterministic']}
index={m.run_key(ce['case'],'main','b',1):rec}
reuse,st=m.run_one(ce,'main',cfg,'b',1,binary,bsha,sid,out,0.00001,index,res);summary['timeout_changed_resume']={'status':st,'stored_timeout':reuse['timeout_s'],'requested_timeout':0.00001}
rt=copy.deepcopy(rec);rt.update(final_classification='replay_timeout',replay_ok=False,replay_exit=-9)
reuse,st=m.run_one(ce,'main',cfg,'b',1,binary,bsha,sid,out,5,{m.run_key(ce['case'],'main','b',1):rt},res);summary['replay_timeout_resume']={'status':st,'final':reuse['final_classification']}
pathlib.Path(rec['artifact_path']).unlink()
reuse,st=m.run_one(ce,'main',cfg,'b',1,binary,bsha,sid,out,5,index,res);summary['missing_artifact_resume']={'status':st,'artifact_exists':pathlib.Path(rec['artifact_path']).exists(),'replay_ok':reuse['replay_ok']}
# A valid stdout without an artifact file is also not replayed or rejected.
m.run_process=lambda cmd,timeout_s,cwd=m.REPO:{'cmd':list(map(str,cmd)),'exit_code':0,'stdout':json.dumps({'outcome':'repaired','counts':{}}),'stderr':'','wall_ms':1,'timeout_s':timeout_s,'timed_out':False}
nop,_=m.run_one(ce,'missing','main' if False else cfg,'b',1,binary,bsha,sid,out,5,None,res);summary['missing_artifact_fresh']={k:nop[k] for k in ['final_classification','replay_ok','artifact_path']};m.run_process=original
# Exercise total-budget enforcement with real short subprocesses, rather than real lengthy searches.
original_plan=m.plan_runs;original_one=m.run_one
m.plan_runs=lambda manifest,suite:iter([(ce,'main',cfg,'b',1)])
def delay_one(*args):
 timeout=args[10];rr=original([sys.executable,'-c','import time; time.sleep(0.25)'],timeout)
 return {'final_classification':'external_timeout' if rr['timed_out'] else 'repaired','wall_ms':rr['wall_ms']},'ran'
m.run_one=delay_one
t=time.monotonic()
with contextlib.redirect_stdout(io.StringIO()):m.do_run(types.SimpleNamespace(bin=str(binary),out=str(p/'deadline'),resume=False,suite=['pilot'],total_budget=0.05,timeout=2))
summary['total_budget']={'budget_s':0.05,'elapsed_s':round(time.monotonic()-t,3),'process_sleep_s':0.25}
m.run_one=original_one;m.plan_runs=original_plan
(p/'runner-probe-summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary,indent=2))
