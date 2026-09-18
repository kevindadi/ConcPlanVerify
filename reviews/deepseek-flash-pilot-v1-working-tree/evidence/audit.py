"""Offline audit: no credentials or network; preserve the delivered pilot."""
import copy
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile

APP = pathlib.Path('/Users/kevin/local-repos/ConcPlanVerify')
PILOT = pathlib.Path('/Users/kevin/paper-review/papers/ConcPlanVerify/experiments/deepseek-flash-pilot-v1')
sys.path.insert(0, str(APP / 'python'))
from cir_workflow.structural import run_structural_check
from cir_workflow.concir_client import _validate_replay_payload

manifest = json.loads((PILOT / 'manifest.json').read_text())
binary = manifest['concir_binary']['path']
checks = []
def check(path, record):
    p = pathlib.Path(path)
    actual = hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None
    checks.append({'path': str(p), 'matches': actual == record['sha256']})

check(binary, manifest['concir_binary'])
check(PILOT / 'TASKS.json', manifest['TASKS.json'])
for group, root in [('application_source', APP), ('prompt_assets', APP),
                    ('contracts', PILOT), ('offline_evidence', PILOT), ('documents', PILOT)]:
    for path, record in manifest[group].items():
        check(root / path, record)

summary = json.loads((PILOT / 'SUMMARY.json').read_text())
batch = pathlib.Path(summary['batch'])
logs = [json.loads(line) for line in (batch / 'llm/llm_requests.jsonl').read_text().splitlines()]
requests = [{k: row.get(k) for k in ['status', 'requested_model', 'response_model',
            'finish_reason', 'transport_attempt', 'thinking', 'usage']} for row in logs]

def cli(*args):
    result = subprocess.run([binary, *map(str, args)], capture_output=True, text=True, timeout=30)
    try:
        payload = json.loads(result.stdout)
    except ValueError:
        payload = None
    return {'exit': result.returncode, 'payload': payload, 'stderr': result.stderr}

tasks = []
for task in json.loads((PILOT / 'TASKS.json').read_text())['tasks']:
    model = next((batch / task['id']).glob('run-*/frozen_initial.cir.json'))
    contract = PILOT / task['contract']
    row = {'id': task['id'], 'check': cli('check', model),
           'explore': cli('explore', model, contract, 'petri')}
    artifact = list((batch / task['id']).glob('calls/*-repair/artifact.json'))
    row['replay'] = cli('replay', artifact[0]) if artifact else None
    tasks.append(row)

# Counterexample to the structural check's advertised modeling-fidelity claim:
# retain both worker definitions but remove the second task from reachable scope.
original = next((batch / 't2_abba').glob('run-*/frozen_initial.cir.json'))
mutant = copy.deepcopy(json.loads(original.read_text()))
entry = mutant['modules'][0]['functions'][0]['body'][0]
assert entry['kind'] == 'scope' and len(entry['funcs']) == 2
entry['funcs'] = entry['funcs'][:1]
with tempfile.TemporaryDirectory(prefix='cpv-fidelity-') as temp:
    path = pathlib.Path(temp) / 'single-task.json'
    path.write_text(json.dumps(mutant))
    probe = {'mutation': 'scope runs only t1; t2 remains unreachable',
             'structural': run_structural_check('abba_inversion', mutant),
             'check': cli('check', path),
             'explore': cli('explore', path, PILOT / 'contracts/t2_abba.json', 'petri')}

artifact = next((batch / 't2_abba').glob('calls/*-repair/artifact.json'))
contradictory = copy.deepcopy(tasks[1]['replay']['payload'])
contradictory.update(accepted_ok=False, nodes=999, outcome='no_acceptable_candidate')
replay_probe = {'payload': contradictory,
                'validation_error': _validate_replay_payload(contradictory, artifact)}
output = {'hash_checks': len(checks), 'hash_mismatches': [x for x in checks if not x['matches']],
          'budget': json.loads((batch / 'budget.json').read_text()),
          'requests': requests, 'tasks': tasks, 'structural_false_positive': probe,
          'replay_inconsistent_payload': replay_probe}
path = pathlib.Path(__file__).with_name('audit.json')
path.write_text(json.dumps(output, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({'hash_checks': len(checks), 'hash_mismatches': output['hash_mismatches'],
    'requests': len(requests), 'total_tokens': sum(r['usage']['total_tokens'] for r in requests),
    'tasks': [{'id': t['id'], 'check': t['check']['payload'],
        'explore': {k: t['explore']['payload'].get(k) for k in ['outcome', 'complete']},
        'replay': t['replay']} for t in tasks],
    'probe_structural_ok': probe['structural']['ok'], 'probe_check': probe['check']['payload']},
    ensure_ascii=False, indent=2))
