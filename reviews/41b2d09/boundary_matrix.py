from pathlib import Path
import json,subprocess,copy
p=Path(__file__).resolve().parent;b='/private/tmp/concir-audit-repair-loop-target/debug/concir-backend';fx=Path('/Users/kevin/local-repos/ConcIR/tests/repro_bench');rows=[]
for st in ['a','b','c']:
 for label,args in [('default',[]),('depth1',['--max-depth','1']),('edits1',['--max-total-edits','1']),('both1',['--max-depth','1','--max-total-edits','1']),('depth0',['--max-depth','0']),('edits0',['--max-total-edits','0'])]:
  name=f'matrix_{st}_{label}';f=p/(name+'.json');r=subprocess.run([b,'repair',str(fx/'preserved_unfixable.json'),str(fx/'preserved_unfixable_contract.json'),'--strategy',st,*args],text=True,capture_output=True);f.write_text(r.stdout);a=json.loads(r.stdout)
  rr=subprocess.run([b,'replay',str(f)],text=True,capture_output=True);(p/(name+'.replay.stdout.json')).write_text(rr.stdout);(p/(name+'.replay.stderr.txt')).write_text(rr.stderr)
  rows.append({'case':name,'search_exit':r.returncode,'outcome':a['outcome'],'stop_reason':a['stop_reason'],'replay_exit':rr.returncode,'replay_error':rr.stderr})
(p/'boundary-matrix-summary.json').write_text(json.dumps(rows,indent=2));print(json.dumps(rows,indent=2))
