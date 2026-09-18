from pathlib import Path
import json,hashlib,collections,subprocess,time
root=Path('/Users/kevin/local-repos/ConcIR');p=root/'experiments/pilot-v1';out=Path('/private/tmp/concir-pilot-audit');rows=[json.loads(s) for s in (p/'results.jsonl').read_text().splitlines()];issues=[];keys=[];cost={st:collections.Counter() for st in ['b','c']}
hashfile=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
for r in rows:
 key=tuple(r[x] for x in ['suite','case','config_name','strategy','repeat']);keys.append(key);a=json.loads(Path(r['artifact_path']).read_text());rd=Path(r['raw_dir'])
 checks={'model_hash':hashfile(r['model'])==r['model_sha256'],'contract_hash':hashfile(r['contract'])==r['contract_sha256'],'artifact_stdout':a==json.loads((rd/'search.stdout').read_text()),'outcome':a['outcome']==r['raw_outcome']==r['final_classification'],'counts':all(a['counts'][ak]==r[rk] for ak,rk in [('proposals','proposals'),('verification_calls','verification_calls'),('states_explored','states_explored'),('cache_hits','cache_hits')]),'search_exit':int((rd/'search.exit').read_text())==r['exit_code'],'replay_exit':int((rd/'replay.exit').read_text())==r['replay_exit']==0,'config':all(a['effective_config'][k]==v for k,v in r['config'].items()),'contract_embed':a['frozen_contract']['bounds']==json.loads(Path(r['contract']).read_text())['bounds']}
 if not all(checks.values()):issues.append({'key':key,'checks':checks})
 if r['suite']=='pilot' and r['config_name']=='main' and r['repeat']==1 and r['strategy'] in cost and r['final_classification']=='repaired':
  for k in ['verification_calls','states_explored','wall_ms']:cost[r['strategy']][k]+=r[k]
manifest=json.loads((p/'manifest.json').read_text());s1=json.loads((p/'cases/p1_same.json').read_text());s2=json.loads((p/'cases/p1_cross.json').read_text());s1.pop('program');s2.pop('program')
caseissues=[]
for c in manifest['cases']:
 m=json.loads((p/c['model']).read_text());w=json.loads((p/c['witness']).read_text());ct=json.loads((p/c['contract']).read_text());wc=json.loads((p/c['witness_contract']).read_text());changes=[]
 for mm,wm in zip(m['modules'],w['modules']):
  for f,wf in zip(mm['functions'],wm['functions']):
   for stmt,wstmt in zip(f['body'],wf['body']):
    if stmt!=wstmt:changes.append({'fn':f['name'],'sid':stmt['sid'],'kind':stmt['kind'],'old':stmt.get('resource'),'new':wstmt.get('resource')})
 caseissues.append({'case':c['case'],'modules':[x['name'] for x in m['modules']],'bounds':ct['bounds']['max_states'],'witness_bounds':wc['bounds']['max_states'],'witness_unlocked_changed':any(x['kind']=='mutex_unlock' for x in changes),'changed_statements':changes if c['case']=='p1_same' else len(changes),'minimal_patch_note':c['minimal_patch_note'] if c['case'].startswith('c_') else None})
selected=[r for r in rows if r['suite']=='pilot' and r['config_name']=='main' and r['repeat']==1 and ((r['case']=='p2_interf' and r['strategy'] in ['b','c']) or (r['case'] in ['p1_same','p1_cross','p2_same','p3_same','p3_interf','c_scope_restricted','c_preserved_unsatisfiable','c_bounds_unknown','c_already_correct'] and r['strategy']=='c'))]
summary={'rows':len(rows),'duplicate_keys':len(keys)-len(set(keys)),'raw_consistency_issues':issues,'totals':{suite:dict(collections.Counter(r['final_classification'] for r in rows if r['suite']==suite)) for suite in ['smoke','pilot']},'main_r1_common_success_cost':{k:dict(v) for k,v in cost.items()},'p1_same_equals_p1_cross_except_program_name':s1==s2,'case_checks':caseissues}
(out/'data-audit.json').write_text(json.dumps(summary,indent=2));replays=[]
for r in selected:
 pr=subprocess.run([str(root/'target/release/concir-backend'),'replay',r['artifact_path']],capture_output=True,text=True,timeout=30);replays.append({'case':r['case'],'strategy':r['strategy'],'exit':pr.returncode,'stdout':pr.stdout,'stderr':pr.stderr});(out/'sample-replays.json').write_text(json.dumps(replays,indent=2))
c=json.loads((p/'cases/p3_same_contract.json').read_text());c['bounds']['max_states']=200000;cp=out/'p3_same_200k_contract.json';cp.write_text(json.dumps(c));t=time.monotonic();pr=subprocess.run([str(root/'target/release/concir-backend'),'explore',str(p/'cases/p3_same.json'),str(cp),'petri'],capture_output=True,text=True,timeout=30);(out/'p3_same_200k.json').write_text(pr.stdout);a=json.loads(pr.stdout);print(json.dumps({'rows':len(rows),'raw_issues':issues,'sample_replays':[(r['case'],r['strategy'],r['exit']) for r in replays],'p1_cross_duplicate':s1==s2,'cost':summary['main_r1_common_success_cost'],'p3_same_200k':{'outcome':a['outcome'],'complete':a['complete'],'states':a['states_explored'],'wall_s':round(time.monotonic()-t,3)}}))
