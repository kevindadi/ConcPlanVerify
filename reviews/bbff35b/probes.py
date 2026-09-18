import json,copy,pathlib,subprocess
out=pathlib.Path(__file__).resolve().parent
root=pathlib.Path('/Users/kevin/local-repos/ConcIR');fx=root/'tests/repro_bench'
binary='/private/tmp/concir-audit-repair-loop-target/debug/concir-backend'
summary={}
def call(name,args):
 r=subprocess.run([binary,*map(str,args)],capture_output=True,text=True,timeout=60)
 (out/(name+'.stdout.json')).write_text(r.stdout);(out/(name+'.stderr.txt')).write_text(r.stderr)
 try:d=json.loads(r.stdout)
 except ValueError:d=None
 return {'exit':r.returncode,'data':d,'stderr':r.stderr}
def repair(name,model='single_cycle',extra=(),contract=None):
 return call(name,['repair',fx/(model+'.json'),contract or fx/(model+'_contract.json'),'--strategy','b',*extra])
s=repair('single');base=s['data'];summary['single']={'exit':s['exit'],'outcome':base['outcome'],'counts':base['counts']}
(out/'valid.json').write_text(json.dumps(base));summary['valid_replay']=call('valid_replay',['replay',out/'valid.json'])
mutations={
 'empty_chain':lambda a:a.update(patch_chain=[]),
 'bad_chain_hash':lambda a:a['patch_chain'][0].update(original_function_hash='bad-hash'),
 'unrelated_accepted':lambda a:a['accepted_program']['modules'][0]['resources'].append({'name':'unrelated','kind':'sync','type':'Mutex','mode':'Sync'}),
 'forbidden_scope':lambda a:a['frozen_contract']['allowed_scope'].update(allow_lock_reorder=False),
 'deleted_preserved':lambda a:a['frozen_contract'].update(preserved=[]),
 'bad_attempt_parent':lambda a:a['attempts'][0].update(parent=99999),
 'false_counts':lambda a:a['counts'].update(verification_calls=0,states_explored=0),
 'false_effective_bounds':lambda a:a['effective_config']['bounds'].update(max_states=1),
 'false_node_complete':lambda a:a['nodes'][-1]['report'].update(complete=False),
 'false_root_properties':lambda a:a['nodes'][0]['report'].update(properties=[]),
 'bad_incoming_result_hash':lambda a:a['nodes'][-1]['incoming'].update(program_fingerprint='bad-hash'),
}
for name,mutate in mutations.items():
 a=copy.deepcopy(base);mutate(a);f=out/(name+'.json');f.write_text(json.dumps(a));summary[name]=call(name+'_replay',['replay',f])
for name,extra in [('verification_zero',['--verification-budget','0']),('verification_one',['--verification-budget','1']),('depth_one',['--max-depth','1']),('edits_one',['--max-total-edits','1']),('verification_seven',['--verification-budget','7'])]:
 r=repair(name,'two_cycles',extra);a=r['data'];summary[name]={'exit':r['exit'],'outcome':a['outcome'],'stop_reason':a['stop_reason'],'counts':a['counts'],'attempts':len(a['attempts']),'nodes':len(a['nodes'])}
for name in ['two_cycles','preserved_unfixable']:
 r=repair(name,name);a=r['data'];summary[name]={'exit':r['exit'],'outcome':a['outcome'],'counts':a['counts'],'ids':[n['id'] for n in a['nodes']],'parents':[n['parent'] for n in a['nodes']]}
for kind,typ,init in [('bounded',{'Int':[0,1]},0),('enum',{'Enum':['a','b']},'a'),('struct',{'Struct':{'x':{'Int':[0,1]}}},{'x':0}),('array',{'Array':{'elem':{'Int':[0,1]},'len':1}},[0])]:
 p=json.loads((fx/'single_cycle.json').read_text());p['modules'][0]['resources'].append({'name':'audit','kind':'var','type':'Var','base':typ,'init':init});f=out/(kind+'.json');f.write_text(json.dumps(p))
 r=call(kind+'_repair',['repair',f,fx/'single_cycle_contract.json','--strategy','c']);a=r['data'];f=out/(kind+'_accepted.json');f.write_text(json.dumps(a['accepted_program']));q=call(kind+'_reload',['explore',f,fx/'single_cycle_contract.json']);summary[kind]={'repair_exit':r['exit'],'reload_exit':q['exit'],'outcome':q['data']['outcome'],'complete':q['data']['complete']}
cp=json.loads((fx/'two_cycles_contract.json').read_text());cp['bounds']['max_states']=1;f=out/'tiny_contract.json';f.write_text(json.dumps(cp));r=repair('tiny','two_cycles',contract=f);a=r['data'];summary['tiny']={'exit':r['exit'],'outcome':a['outcome'],'counts':a['counts'],'bounds':a['effective_config']['bounds']}
(out/'summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary,indent=2))
