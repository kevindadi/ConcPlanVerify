"""Python orchestration layer for LLM-driven CIR generation and repair.

The package owns model interaction and repair-loop control. Rust remains the
source of truth for CIR validation, translation, and verification.
"""

from .generation import GenerationResult, GenerationWorkflow
from .env import load_dotenv
from .llm import (
    DeepSeekClient,
    LlmClient,
    LlmError,
    QwenClient,
    create_llm_client,
    default_base_url,
)
from .models import ModelConfig, RepairResult, RustCliResult
from .repair import RepairWorkflow
from .rust_cli import RustCli

__all__ = [
    "GenerationResult",
    "GenerationWorkflow",
    "DeepSeekClient",
    "LlmClient",
    "LlmError",
    "ModelConfig",
    "QwenClient",
    "RepairResult",
    "RepairWorkflow",
    "RustCli",
    "RustCliResult",
    "create_llm_client",
    "default_base_url",
    "load_dotenv",
]
