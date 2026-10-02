#!/usr/bin/env python3
"""Offline controls for the v23 thinking-batch fixes. Never calls an LLM.

Freeze the old source/CIR/contract identities, re-evaluate with explicit tools,
then reject source mutants with mismatched semaphore counts or channel capacities. Original
experiment data is read-only; --out must be a new directory.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / 'python'))
from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--batch', type=Path, required=True)
    parser.add_argument('--tools', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    batch, tools, out = args.batch.resolve(), args.tools.resolve(), args.out.resolve()
    if out == batch or batch in out.parents:
        parser.error('--out must be outside the frozen batch')
    out.mkdir(parents=True, exist_ok=False)
    rows = []
    cases = [('semaphore__acquire_twice_no_release', 'gpt-6-luna', 'Semaphore::new(1)', 'Semaphore::new(2)'),
             ('semaphore__acquire_twice_no_release', 'glm-5.3-flash', 'Semaphore::new(1)', 'Semaphore::new(2)'),
             ('channel__bounded_backpressure_lock_held', 'glm-5.3-flash', 'sync_channel::<i32>(1)', 'sync_channel::<i32>(2)')]
    for task, model, constructor, wrong_constructor in cases:
        old_path = batch / task / model / 'G3_concir/rep0/code/round-1/result.json'
        old = json.loads(old_path.read_text())
        source_path, cir, contract = (Path(old[key]) for key in ('source_path', 'cir_path', 'contract_path'))
        for path, key in [(source_path, 'source_sha256'), (cir, 'cir_sha256'), (contract, 'contract_sha256')]:
            if sha(path) != old[key]:
                raise RuntimeError(f'frozen identity changed: {path}')
        source = source_path.read_text()
        if source.count(constructor) != 1:
            raise RuntimeError('expected exactly one known constructor')
        for label, code in [('unchanged_candidate', source),
                            ('wrong_initial_count_control', source.replace(constructor, wrong_constructor))]:
            work = out / task / model / label
            r = evaluate_candidate(code, cir, contract, work, binary=tools / 'concir-backend',
                                   instrument=tools / 'concir-instrument', binding_binary=tools / 'bind_check',
                                   n_runs=32, cell_id=f'offline-debug:{task}:{model}:{label}')
            row = {'task': task.replace('__', '/'), 'model': model, 'kind': label, 'old_result': str(old_path),
                   'old_result_sha256': sha(old_path), 'old_verdict': old['ledger']['current_evaluation'],
                   'source_changed': label != 'unchanged_candidate', 'result': str(work / 'result.json'),
                   'new_verdict': r['ledger']['current_evaluation'], 'followup': r['followup'],
                   'runs_completed': r.get('runs_completed'), 'reasons': r['ledger']['reasons']}
            rows.append(row)
            print(json.dumps(row, ensure_ascii=False), flush=True)
    summary = {'real_llm_requests': 0, 'frozen_batch': str(batch), 'tools': str(tools),
               'note': 'Derived offline checker controls, not new model generations or corrected historical scores.',
               'rows': rows}
    (out / 'CONTROLS.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n')
    positives = [r for r in rows if not r['source_changed']]
    negatives = [r for r in rows if r['source_changed']]
    return 0 if (all(r['new_verdict'] == 'satisfied_bounded' for r in positives)
                 and all(r['new_verdict'] != 'satisfied_bounded' and r['followup']['action'] == 'repair'
                         for r in negatives)) else 1


if __name__ == '__main__':
    raise SystemExit(main())
