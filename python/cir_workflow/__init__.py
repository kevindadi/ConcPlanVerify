"""Python orchestration layer for LLM-driven ConcIR generation and repair.

The package owns model interaction, prompt assets, and generation/repair
orchestration. The Rust ``concir-backend`` CLI remains the source of truth for
CIR validation, supportability, exploration/verification, diagnosis,
deterministic repair and artifact replay; this package only speaks its protocol.
"""

from .concir_client import ConcirClient, ConcirIdentity, ConcirResult
from .env import load_dotenv
from .llm import DeepSeekClient, LlmClient, LlmError, QwenClient, create_llm_client, default_base_url
from .models import ModelConfig

__all__ = [
    "ConcirClient",
    "ConcirIdentity",
    "ConcirResult",
    "DeepSeekClient",
    "LlmClient",
    "LlmError",
    "ModelConfig",
    "QwenClient",
    "create_llm_client",
    "default_base_url",
    "load_dotenv",
]
