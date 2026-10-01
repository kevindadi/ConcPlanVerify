#!/usr/bin/env python3
"""Mock dry-run of the four audited transports. No network and no API keys."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog  # noqa: E402
from cir_workflow.channels import AuditedClient  # noqa: E402
from cir_workflow.transport import build_registry, require_experiment_model  # noqa: E402

MODELS = (
    ("DeepSeek Flash", {"max_tokens": 4096, "temperature": 0, "model": "deepseek-flash",
                        "base_url": "https://api.deepseek.com", "extra_body": None,
                        "reasoning_effort": None}),
    ("Qwen", {"max_tokens": 4096, "temperature": 0, "model": "qwen3.8-flash",
              "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
              "extra_body": {"enable_thinking": False}, "reasoning_effort": None}),
    ("GPT 6 Luna", {"max_tokens": 4096, "temperature": 0, "model": "gpt-6-luna",
                    "base_url": "https://opencode.ai/zen/go/v1", "extra_body": None,
                    "reasoning_effort": None}),
    ("GLM 5.3 Flash", {"max_tokens": 16384, "temperature": 0, "model": "glm-5.3-flash",
                       "base_url": "https://opencode.ai/zen/go/v1",
                       "extra_body": None, "reasoning_effort": "low"}),
)


class _Fake:
    def __init__(self, params: dict):
        self.calls = 0
        for key, value in params.items():
            setattr(self, key, value)

    def complete(self, system, user):
        self.calls += 1
        return SimpleNamespace(
            text="fn main() {}", response_model=self.model,
            usage={"prompt_tokens": 1, "completion_tokens": 1},
            wall_ms=1, finish_reason="stop", request_id="dry", transport_attempt=1)


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    config = json.loads(args.config.read_text(encoding="utf-8"))
    if config.get("real_requests_authorized"):
        print("config authorizes real requests; dry-run refuses")
        return 2
    args.output.mkdir(parents=True, exist_ok=True)
    records = []
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        for display, params in MODELS:
            spec = require_experiment_model(build_registry(), display)
            inner = _Fake(params)
            client = AuditedClient(
                inner, audit=AuditLog(root / f"{spec.model_id}.jsonl"), run_id="dry",
                cell_id=f"dry-{spec.model_id}", spec=spec, arm="G0_direct",
                task_id="atomic-data/atomic_lost_update", replicate=0, stage="code")
            protocol = client.protocol_record()
            if not protocol.get("max_tokens"):
                print("wrapper protocol is empty", display)
                return 2
            if "api_key" in json.dumps(protocol):
                print("protocol contains a key field")
                return 2
            client.complete("system", "user")
            records.append({
                "display_name": display,
                "model_id": spec.model_id,
                "channel": spec.channel,
                "surface": spec.surface,
                "protocol_record": protocol,
                "fake_calls": inner.calls,
                "real_requests": 0,
            })
    out = {
        "real_requests": 0,
        "fake_calls": sum(item["fake_calls"] for item in records),
        "models": records,
        "config_status": config.get("status"),
        "started_paid_experiment": False,
    }
    (args.output / "DRY_RUN.json").write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"real_requests": 0, "fake_calls": out["fake_calls"],
                      "models": [item["model_id"] for item in records]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
