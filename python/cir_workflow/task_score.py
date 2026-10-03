"""Pick the independent requirement scorer from the task id.

Channel tasks do not use the condvar scorer, and the condvar task does not
fall through to the channel scorer. An unknown task is unknown, not a pass.
"""

from __future__ import annotations

from pathlib import Path

from .condvar_requirements import TASK_ID as CONDVAR_TASK
from .condvar_requirements import evaluate_condvar
from .send_holding_requirements import evaluate_requirements

CHANNEL_TASK = "channel/send_while_holding_mutex"


def score_for_task(task_id: str, source: str, work: Path) -> dict:
    if task_id == CHANNEL_TASK:
        result = evaluate_requirements(source, work)
        result["task_id"] = task_id
        return result
    if task_id == CONDVAR_TASK:
        return evaluate_condvar(source, work)
    return {"status": "unknown", "task_id": task_id,
            "reason": "no independent scorer for this task", "checks": {}, "runs": []}
