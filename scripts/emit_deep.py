import json
import sys
from pathlib import Path

sys.path.insert(0, "python")
from cir_workflow.scaling import lock_chain_deep

root = Path(".")


def write(path: Path, program: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(program, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("wrote", path)


buggy = lock_chain_deep(4, 3, buggy=True)
write(root / "benchmarks/cir/deep_lock_chain_4x3/buggy.json", buggy)

fixed = lock_chain_deep(4, 3, buggy=False)
fixed["program"] = "deep_lock_chain_4x3"
write(root / "benchmarks/cir/deep_lock_chain_4x3/fixed.json", fixed)

safe_twin = lock_chain_deep(4, 3, buggy=False)
write(root / "benchmarks/cir/deep_lock_chain_4x3_safe/buggy.json", safe_twin)
