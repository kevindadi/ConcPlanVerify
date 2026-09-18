import sys,json,hashlib,subprocess,re
from pathlib import Path
sys.dont_write_bytecode=True
repo=Path('/Users/kevin/local-repos/ConcIR');root=Path('/private/tmp/concir-lifecycle-review');base=repo/'experiments/pilot-v2-lifecycle'
sys.path.insert(0,str(repo/'scripts'));import pilot_audit;import run_pilot as r
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
result={'audit':pilot_audit.audit(base),'batches':[],'replays':[]}
assert not result['audit']['issues']
for bd in (base/'results/batches').iterdir():
 if not (bd/'valid_index.jsonl').exists():continue
 rs=[json.loads(x) for x in (bd/'valid_index.jsonl').read_text().splitlines()]
 plan=json.loads((bd/'plan.json').read_text());env=json.loads((bd/'environment.json').read_text())
 assert {x['run_key'] for x in rs}=={x['run_key'] for x in plan}
 assert len(rs)==len(plan)
 for f,v in env['code']['files'].items():
  if v['sha256']:
   p=Path(f[9:]) if f.startswith('external:') else repo/f
   assert sha(p)==v['sha256'],f
   assert sha(bd/'code_snapshot'/p.name)==v['sha256'],f
 for x in rs:
  ad=Path(x['attempt_dir'])
  if x['stage']=='repair':
   a=json.loads(Path(x['artifact_path']).read_text());assert sha(x['artifact_path'])==x['artifact_sha256']
   assert int((ad/'search.exit').read_text())==x['exit_code']==r.REPAIR_EXIT[a['outcome']]
   assert int((ad/'replay.exit').read_text())==x['replay_exit']==0
   for kind,field in [('program','model'),('contract','contract')]:
    p=x['identity'][field+'_path'];assert sha(p)==x['identity'][field+'_sha256']
    norm=json.loads(subprocess.check_output([str(repo/'target/release/examples/pilot_tool'),'normalize',p,'--kind',kind]))
    assert norm==a['input_program' if kind=='program' else 'frozen_contract']
   assert all(a['effective_config'][k]==v for k,v in x['config'].items())
   q=subprocess.run([str(repo/'target/release/concir-backend'),'replay',x['artifact_path']],text=True,capture_output=True,timeout=30)
   assert q.returncode==0
   result['replays'].append({'case':x['case'],'strategy':x['strategy'],'exit':q.returncode})
  else:
   a=json.loads((ad/'explore.stdout').read_text());assert a['outcome']==x['raw_outcome'] and a['complete']==x['report_complete']
   assert a['states_explored']==x['states_explored']
 result['batches'].append({'batch':bd.name,'records':len(rs),'complete':sum(x['evidence_status']=='complete' for x in rs)})
lines=(base/'frozen-data-sha256.txt').read_text().splitlines();bad=[]
for line in lines:
 h,p=line.split(None,1)
 if sha(repo/p)!=h:bad.append(p)
result['frozen_manifest']={'files':len(lines),'mismatches':bad}
old=Path('/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/pilot-v1-working-tree/delivery-sha256.json');previous=json.loads(old.read_text());result['v1_prior_review_mismatches']=[p for p,h in previous.items() if p.startswith('experiments/pilot-v1/') and sha(repo/p)!=h]
s=Path('/private/tmp/concir-lifecycle-core-tests.log').read_text();result['core_tests']={'passed':sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))),'warnings':len(re.findall('^warning:',s,re.M))}
(root/'delivery-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['audit','replays']},indent=2));print('fresh replay:',len(result['replays']))
