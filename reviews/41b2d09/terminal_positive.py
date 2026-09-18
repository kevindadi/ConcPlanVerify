from pathlib import Path
import json,subprocess
p=Path(__file__).resolve().parent;fx=Path('/Users/kevin/local-repos/ConcIR/tests/repro_bench');binary='/private/tmp/concir-audit-repair-loop-target/debug/concir-backend';rows=[]
cases=[('already_correct',['repair',str(fx/'already_correct.json'),str(fx/'already_correct_contract.json'),'--strategy','b'])]
for name,artifact in [('invalid','b_invalid.stdout.json'),('unsupported','b_unsupported.stdout.json'),('unknown','tiny.stdout.json'),('budget_zero','verification_zero.stdout.json'),('budget_one','verification_one.stdout.json'),('denied','modules_denied.stdout.json'),('unfixable','preserved_unfixable.stdout.json'),('reused','two_cycles.stdout.json')]:cases.append((name,artifact))
for name,source in cases:
 if isinstance(source,list):
  r=subprocess.run([binary,*source],text=True,capture_output=True);f=p/(name+'.stdout.json');f.write_text(r.stdout)
 else:f=p/source
 r=subprocess.run([binary,'replay',str(f)],text=True,capture_output=True,timeout=60)
 try:data=json.loads(r.stdout)
 except ValueError:data=None
 rows.append({'case':name,'exit':r.returncode,'result':data,'stderr':r.stderr})
(p/'terminal-positive-summary.json').write_text(json.dumps(rows,indent=2));print(json.dumps(rows,indent=2))
