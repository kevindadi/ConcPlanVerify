"""v23 spec resolution: the transport and audit use the CONFIG channel, not the
registry default (the v22 path mislabeled Qwen as dashscope-direct)."""

from __future__ import annotations

from .transport import ModelSpec

PROVIDERS = {"qwen3.8-flash": "qwen", "gpt-6-luna": "openai",
             "glm-5.3-flash": "zhipu"}


def config_spec(entry: dict) -> ModelSpec:
    return ModelSpec(entry["display_name"], PROVIDERS.get(entry["model_id"], "opencode"),
                     entry["channel"], entry["model_id"],
                     surface=entry.get("surface", "chat"), status="available",
                     discovered=True)
