from pathlib import Path
import json,copy,subprocess
p=Path(__file__).resolve().parent;b='/private/tmp/concir-audit-repair-loop-target/debug/concir-backend';rows={}
def probe(name,file,change):
 a=json.loads((p/file).read_text());change(a);f=p/(name+'.json');f.write_text(json.dumps(a));r=subprocess.run([b,'replay',str(f)],capture_output=True,text=True)
 (p/(name+'.stdout.json')).write_text(r.stdout);(p/(name+'.stderr.txt')).write_text(r.stderr)
 rows[name]={'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
probe('wrong_truncation','matrix_b_depth1.json',lambda a:a.update(truncation=None))
probe('fake_truncation','matrix_b_default.json',lambda a:a.update(truncation='max-depth'))
probe('wrong_priority','matrix_b_both1.json',lambda a:a.update(stop_reason='max-total-edits'))
probe('root_unknown_flag_false','tiny.stdout.json',lambda a:a.update(saw_unknown=False))
(p/'terminal-flags-summary.json').write_text(json.dumps(rows,indent=2));print(json.dumps(rows,indent=2))
