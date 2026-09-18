from pathlib import Path
import json,copy,subprocess
p=Path(__file__).resolve().parent
binary='/private/tmp/concir-audit-repair-loop-target/debug/concir-backend'
rows={}
def probe(name,base,fn):
 a=copy.deepcopy(base);fn(a);f=p/(name+'.json');f.write_text(json.dumps(a));r=subprocess.run([binary,'replay',str(f)],capture_output=True,text=True,timeout=60)
 (p/(name+'.stdout.json')).write_text(r.stdout);(p/(name+'.stderr.txt')).write_text(r.stderr)
 try:d=json.loads(r.stdout)
 except ValueError:d=None
 rows[name]={'exit':r.returncode,'result':d,'stderr':r.stderr}
single=json.loads((p/'single.stdout.json').read_text())
probe('attempt_bad_hash',single,lambda a:a['attempts'][0]['patch'].update(original_hash='broken'))
probe('attempt_missing_sid',single,lambda a:a['attempts'][0]['patch']['changes'][0].update(a='nonexistent_sid'))
probe('attempt_wrong_target',single,lambda a:a['attempts'][0]['patch'].update(function='nonexistent_function'))
probe('false_transitions',single,lambda a:a['nodes'][0]['report'].update(transitions_explored=0))
def zero_states(a):
 for n in a['nodes']:n['report']['states_explored']=0
 a['counts']['states_explored']=0
 a['accepted_report']['states_explored']=0
probe('zero_states_consistently',single,zero_states)
probe('false_analysis_started',single,lambda a:a['nodes'][0]['report'].update(analysis_started=False))
probe('erase_counterexample',single,lambda a:a['nodes'][0]['report']['diagnostics'][0].update(counterexample=[]))
probe('erase_blocking_facts',single,lambda a:a['nodes'][0]['report']['diagnostics'][0].update(blocked=[]))
blocked=json.loads((p/'verification_one.stdout.json').read_text())
probe('blocked_patch_bad_hash',blocked,lambda a:a['attempts'][0]['patch'].update(original_hash='broken'))
probe('budget_as_no_candidate',blocked,lambda a:a.update(outcome='no_acceptable_candidate'))
probe('budget_reason_solved',blocked,lambda a:a.update(stop_reason='solved'))
unfix=json.loads((p/'preserved_unfixable.stdout.json').read_text())
probe('unfix_as_unknown',unfix,lambda a:a.update(outcome='analysis_unknown'))
probe('unfix_as_budget',unfix,lambda a:a.update(outcome='budget_exhausted'))
probe('unfix_wrong_stop_reason',unfix,lambda a:a.update(stop_reason='verification-budget'))
(p/'residual-summary.json').write_text(json.dumps(rows,indent=2));print(json.dumps(rows,indent=2))
