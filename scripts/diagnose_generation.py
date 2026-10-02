#!/usr/bin/env python3
"""Read a live generation run without trusting its possibly stale SUMMARY.

No model calls, no experiment writes. Arm stopping/acceptance is reported apart
from code requirement evidence. Use --config for the planned cell count.
"""
from __future__ import annotations

import argparse
import collections
import datetime
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / 'python'))
from cir_workflow.multi_gen import read_cell_cache  # noqa: E402


def read_report(root: Path, config: dict | None = None) -> dict:
    rows, warnings = [], []
    for path in sorted(root.glob('*/*/G*/rep*/cache.json')):
        cell = path.parent
        try:
            cache = read_cell_cache(cell)
        except (OSError, ValueError) as exc:
            warnings.append({'path': str(path), 'reason': type(exc).__name__})
            continue
        if not cache or cache.get('status') != 'executed':
            warnings.append({'path': str(path), 'reason': 'cache_not_verified'})
            continue
        task, model, arm, rep = cell.relative_to(root).parts
        row = {'task': task.replace('__', '/'), 'model': model, 'arm': arm, 'rep': rep,
               'arm_accepted': cache.get('accepted'), 'arm_status': cache.get('record_status')}
        evaluations = sorted(cell.glob('code/round-*/result.json'),
                             key=lambda p: int(p.parent.name.split('-')[-1]))
        if arm == 'G3_concir' and evaluations:
            try:
                result = json.loads(evaluations[-1].read_text())
                ledger = result.get('ledger') or {}
                row.update({'code_rounds': len(evaluations),
                            'model_verified': (ledger.get('model') or {}).get('verified'),
                            'model_required_properties': (ledger.get('model') or {}).get('required'),
                            'code_verdict': ledger.get('current_evaluation'),
                            'delivery_status': ledger.get('delivery_status'),
                            'trace_state': (ledger.get('trace') or {}).get('state'),
                            'runs_completed': result.get('runs_completed'),
                            'reasons': ledger.get('reasons'),
                            'requirements': [{k: p.get(k) for k in (
                                'property_id', 'requirements', 'model_verdict',
                                'independent_requirement_result', 'obligation_state')}
                                for p in ledger.get('properties') or []]})
            except (OSError, ValueError) as exc:
                warnings.append({'path': str(evaluations[-1]), 'reason': type(exc).__name__})
        rows.append(row)
    groups = collections.defaultdict(list)
    for row in rows:
        groups[(row['model'], row['arm'])].append(row)
    summary = []
    for (model, arm), cells in sorted(groups.items()):
        summary.append({'model': model, 'arm': arm, 'completed_cells': len(cells),
                        'arm_accepted': sum(c.get('arm_accepted') is True for c in cells),
                        'model_verified': sum(c.get('model_verified') is True for c in cells),
                        'code_verdicts': dict(collections.Counter(
                            c.get('code_verdict', 'not_acquired') for c in cells))})
    planned = None
    unsupported = []
    if config:
        planned = len(config['models']) * len(config['tasks']) * len(config['arms']) * len(config.get('reps', [0]))
        # Existing monitor intentionally cannot observe internal value predicates.
        def value_goals(value):
            if isinstance(value, dict):
                if value.get('kind') in {'var_eq', 'var_cmp', 'var_ref'}:
                    yield value
                for child in value.values():
                    yield from value_goals(child)
            elif isinstance(value, list):
                for child in value:
                    yield from value_goals(child)
        for task in config['tasks']:
            task_id = task['task'] if isinstance(task, dict) else task
            path = REPO / 'benchmarks/families' / task_id / 'contract.json'
            if not path.is_file():
                continue
            contract = json.loads(path.read_text())
            for prop in (contract.get('properties') or []) + (contract.get('preserved') or []):
                goals = list(value_goals(prop))
                if goals:
                    unsupported.append({'task': task_id, 'property': prop.get('id') or prop.get('description'),
                                        'requirements': prop.get('req'), 'goals': goals,
                                        'code_monitor_support': 'unsupported',
                                        'model_support': 'separate; inspect explore evidence'})
    return {'captured_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'run': str(root.resolve()), 'snapshot': 'non_atomic_live_read',
            'planned_cells': planned, 'completed_cells': len(rows),
            'summary_source': 'verified_per_cell_cache; SUMMARY.json deliberately ignored',
            'warning': 'Arm acceptance criteria differ; these counts are not a shared requirement-success score. '
                       'Model PASS is not code-level requirement satisfaction. Missing cells are pending, not failed.',
            'groups': summary, 'cells': rows, 'known_value_monitor_gaps': unsupported,
            'read_warnings': warnings}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', type=Path)
    parser.add_argument('--config', type=Path)
    args = parser.parse_args()
    config = json.loads(args.config.read_text()) if args.config else None
    print(json.dumps(read_report(args.run.resolve(), config), ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
