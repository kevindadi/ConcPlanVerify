import json,pathlib,subprocess,copy
p=pathlib.Path(__file__).resolve().parent;fx=pathlib.Path('/Users/kevin/local-repos/ConcIR/tests/repro_bench');b='/private/tmp/concir-audit-repair-loop-target/debug/concir-backend';rows={}
def run(name,args):
 r=subprocess.run([b,*map(str,args)],text=True,capture_output=True);(p/(name+'.stdout.json')).write_text(r.stdout);(p/(name+'.stderr.txt')).write_text(r.stderr)
 try:a=json.loads(r.stdout)
 except ValueError:a=None
 rows[name]={'exit':r.returncode,'outcome':a.get('outcome') if a else None,'stderr':r.stderr}
 return a
base=json.loads((p/'single.stdout.json').read_text())
for name,mutation in [('bad_node_base',lambda a:a['nodes'][-1]['incoming'].update(original_function_hash='bad')),('bad_input',lambda a:a['input_program'].update(program='changed'))]:
 a=copy.deepcopy(base);mutation(a);f=p/(name+'.json');f.write_text(json.dumps(a));run(name+'_replay',['replay',f])
f=p/'forbidden_contract.json';f.write_text(json.dumps(json.loads((p/'forbidden_scope.json').read_text())['frozen_contract']));a=run('forbidden_fresh_search',['repair',fx/'single_cycle.json',f,'--strategy','b']);rows['forbidden_fresh_search']['counts']=a['counts']
f=p/'modules_denied_contract.json';c=copy.deepcopy(base['frozen_contract']);c['allowed_scope']['modules']=['other'];f.write_text(json.dumps(c));a=run('modules_denied',['repair',fx/'single_cycle.json',f,'--strategy','b']);rows['modules_denied']['nodes']=len(a['nodes']);rows['modules_denied']['attempts']=[{'parent':x['parent'],'result':x['result']} for x in a['attempts']]
a=run('two_cycles_c',['repair',fx/'two_cycles.json',fx/'two_cycles_contract.json','--strategy','c']);rows['two_cycles_c']['counts']=a['counts']
patch=[{'module':'main','function':'t1','changes':[{'kind':'swap_statements','a':'s1','b':'s2'}]}];f=p/'patches.json';f.write_text(json.dumps(patch))
for budget in ['0','1','bad']:
 a=run('legacy_'+budget,['repair',fx/'single_cycle.json',fx/'single_cycle_contract.json',f,budget]);
 if a:rows['legacy_'+budget]['candidates_tried']=a['candidates_tried']
for st in ['a','b','c']:
 for name,c in [('unsupported',{**base['frozen_contract'],'assumptions':{'sequential_consistency':False,'no_spurious_wakeups':True}}),('invalid',{**base['frozen_contract'],'bounds':{**base['frozen_contract']['bounds'],'max_threads':0}})]:
  f=p/(name+'_contract.json');f.write_text(json.dumps(c));run(st+'_'+name,['repair',fx/'single_cycle.json',f,'--strategy',st])
(p/'positive-summary.json').write_text(json.dumps(rows,indent=2));print(json.dumps(rows,indent=2))
