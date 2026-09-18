import sys,json,hashlib,subprocess,collections,time
from pathlib import Path
sys.dont_write_bytecode=True
src=Path('/Users/kevin/local-repos/ConcIR');root=Path('/private/tmp/concir-pilot-v2-audit');base=src/'experiments/pilot-v2'
sys.path.insert(0,str(src/'scripts'))
import run_pilot as r
import pilot_audit
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
issues=[];batches=[];records=[];norm={}
for bd in sorted((base/'results/batches').iterdir()):
    if not (bd/'valid_index.jsonl').exists():continue
    rs=[json.loads(l) for l in (bd/'valid_index.jsonl').read_text().splitlines() if l.strip()]
    ats=[json.loads(l) for l in (bd/'attempts.jsonl').read_text().splitlines() if l.strip()]
    plan=json.loads((bd/'plan.json').read_text());env=json.loads((bd/'environment.json').read_text());sm=json.loads((bd/'summary.json').read_text())
    assert len(rs)==len(plan)==len(ats)==len({x['run_key'] for x in rs})
    assert {x['run_key'] for x in rs}=={x['run_key'] for x in plan}
    for name,item in env['code']['files'].items():
        assert h(src/name)==item['sha256'],name
        assert h(bd/'code_snapshot'/Path(name).name)==item['sha256'],name
    assert h(src/'target/release/concir-backend')==env['binary_sha256']
    assert h(src/'target/release/examples/pilot_tool')==env['tool_sha256']
    for rec in rs:
        for typ,kind in [('model','program'),('contract','contract')]:
            p=rec['identity'][typ+'_path']
            assert h(p)==rec['identity'][typ+'_sha256']
            if p not in norm:
                norm[p]=json.loads(subprocess.check_output([str(src/'target/release/examples/pilot_tool'),'normalize',p,'--kind',kind]))
            assert r.sha_canonical(norm[p])==rec['identity'][typ+'_norm_sha256']
        if rec['stage']=='repair' and rec['evidence_status']=='complete':
            art=Path(rec['artifact_path']);a=json.loads(art.read_text());ad=art.parent
            assert h(art)==rec['artifact_sha256']
            assert a['input_program']==norm[rec['identity']['model_path']]
            assert a['frozen_contract']==norm[rec['identity']['contract_path']]
            cfg={**rec['config'],'strategy':r.STRATEGY_ENUM[rec['strategy']],'bounds':rec['identity']['contract_bounds']}
            assert all(a['effective_config'][k]==v for k,v in cfg.items())
            assert int((ad/'search.exit').read_text())==rec['exit_code']==r.REPAIR_EXIT[a['outcome']]
            assert int((ad/'replay.exit').read_text())==0 and rec['replay_ok']
            assert a['outcome']==rec['final_classification']
        elif rec['stage']=='explore':
            d=json.loads((Path(rec['attempt_dir'])/'explore.stdout').read_text())
            assert d['outcome']==rec['raw_outcome'] and d['states_explored']==rec['states_explored']
    count=dict(collections.Counter(x['final_classification'] for x in rs))
    batches.append(dict(batch=bd.name,records=len(rs),unique_attempt_dirs=len({x['attempt_dir'] for x in ats}),outcomes=count,paired=sm['paired']))
    records.extend(rs)
# Independently compute B/C pairing over current main pilot, and count controls separately.
pairs=collections.defaultdict(dict)
for x in records:
    if x['suite']=='pilot' and x['strategy'] in ('b','c'):
        pairs[(x['case'],x['config_name'],x['repeat'])][x['strategy']]=x
both=[p for p in pairs.values() if len(p)==2 and all(x['final_classification']=='repaired' for x in p.values())]
metrics={'pairs':len(pairs),'both_repaired':len(both),'r1_both':sum(p['b']['repeat']==1 for p in both)}
for st in ['b','c']:
    metrics[st]={k:sum(p[st][k] for p in both) for k in ['verification_calls','states_explored','wall_ms']}
result={'batches':batches,'independent_paired':metrics,'delivered_auditor':pilot_audit.audit(base),'identities_checked':len(norm),'issues':issues}
(root/'delivery-audit.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'batches':[{k:v for k,v in x.items() if k!='paired'} for x in batches],'paired':metrics,'auditor_issues':result['delivered_auditor']['issues'],'identity_checks':len(norm)},indent=2))
# Fresh sampled replays: eight ordinary/control, plus heavy C search tree.
chosen=[]
for case,st in [('p1_cross_shared','b'),('p2_cross_shared','c'),('p2_interf','c'),('e_module_order','b'),('c_scope_restricted','c'),('c_preserved_unsatisfiable','c'),('c_bounds_unknown','b'),('c_already_correct','b')]:
    chosen.append(next(x for x in records if x['suite']=='pilot' and x['case']==case and x['strategy']==st and x['config_name']=='main' and x['repeat']==1))
chosen.append(next(x for x in records if x['suite']=='heavy' and x['strategy']=='c'))
results=[]
for rec in chosen:
    t=time.monotonic();p=subprocess.run([str(src/'target/release/concir-backend'),'replay',rec['artifact_path']],capture_output=True,text=True,timeout=180)
    x=dict(case=rec['case'],suite=rec['suite'],strategy=rec['strategy'],exit=p.returncode,wall_s=round(time.monotonic()-t,3),stdout=p.stdout,stderr=p.stderr)
    results.append(x);print('replay',rec['case'],rec['suite'],p.returncode,x['wall_s'],flush=True)
    (root/'fresh-replay.json').write_text(json.dumps(results,indent=2)+'\n')
    assert p.returncode==0,x
