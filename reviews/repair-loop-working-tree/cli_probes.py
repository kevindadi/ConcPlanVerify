import json, subprocess, pathlib, collections
root=pathlib.Path('/Users/kevin/local-repos/ConcIR')
out=pathlib.Path('/private/tmp/concir-audit-repair-loop')
bin='/private/tmp/concir-audit-repair-loop-target/debug/concir-backend'
fixtures=root/'tests/repro_bench'
summary={}
def run(name,args):
 p=subprocess.run([bin,*map(str,args)],capture_output=True,text=True)
 (out/(name+'.stdout.json')).write_text(p.stdout)
 (out/(name+'.stderr.txt')).write_text(p.stderr)
 try: data=json.loads(p.stdout)
 except: data=None
 return p.returncode,data,p.stderr
for kind,base,init in [('bounded',{'Int':[0,1]},0),('enum',{'Enum':['a','b']},'a'),('struct',{'Struct':{'x':{'Int':[0,1]}}},{'x':0}),('array',{'Array':{'elem':{'Int':[0,1]},'len':1}},[0])]:
 model=json.loads((fixtures/'single_cycle.json').read_text())
 model['modules'][0]['resources'].append({'name':'audit_data','kind':'var','type':'Var','base':base,'init':init})
 path=out/(kind+'.json');path.write_text(json.dumps(model))
 code,data,err=run(kind+'_repair',['repair',path,fixtures/'single_cycle_contract.json','--strategy','c'])
 row={'repair_exit':code,'outcome':data.get('outcome') if data else None,'stderr':err}
 if data and data.get('accepted_program'):
  exported=out/(kind+'_accepted.json');exported.write_text(json.dumps(data['accepted_program']))
  row['exported_base']=data['accepted_program']['modules'][0]['resources'][-1]['base']
  ec,ed,ee=run(kind+'_reload',['explore',exported,fixtures/'single_cycle_contract.json'])
  row.update(reload_exit=ec,reload_stderr=ee)
 summary[kind]=row
for case in ['two_cycles','preserved_unfixable']:
 code,data,err=run(case+'_b',['repair',fixtures/(case+'.json'),fixtures/(case+'_contract.json'),'--strategy','b'])
 nodes=data['nodes'];counts=collections.Counter(n['fingerprint'] for n in nodes)
 parents={n['id']:n for n in nodes}
 bad=[{'id':n['id'],'depth':n['depth'],'parent':n['parent'],'parent_depth':parents[n['parent']]['depth']} for n in nodes if n['parent'] is not None and parents[n['parent']]['depth']+1!=n['depth']]
 summary[case]={'outcome':data['outcome'],'verifications':data['verifications'],'unique_fingerprints':len(counts),'duplicate_fingerprints':{k:v for k,v in counts.items() if v>1},'bad_parent_depths':bad}
contract=json.loads((fixtures/'single_cycle_contract.json').read_text())
contract['allowed_scope']['modules']=['other_module']
cp=out/'denied_modules_contract.json';cp.write_text(json.dumps(contract))
code,data,err=run('denied_modules',['repair',fixtures/'single_cycle.json',cp,'--strategy','b'])
summary['denied_modules']={'outcome':data['outcome'],'nodes':data['nodes']}
contract=json.loads((fixtures/'single_cycle_contract.json').read_text()); contract['assumptions']['sequential_consistency']=False
cp=out/'unsupported_contract.json';cp.write_text(json.dumps(contract))
for cmd in ['explore','repair']:
 args=[cmd,fixtures/'single_cycle.json',cp]+(['--strategy','c'] if cmd=='repair' else [])
 code,data,err=run('unsupported_'+cmd,args)
 summary['unsupported_'+cmd]={'exit':code,'outcome':data['outcome'],'nodes':data.get('nodes')}
(out/'cli_summary.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2))
