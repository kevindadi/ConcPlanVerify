"""Subprocess client subpackage for ``concir-backend``."""

from .client import ConcirClient
from .protocol import (
    CHECK_EXIT,
    CHECK_REQ_EXIT,
    EXPLAIN_CONTRACT_EXIT,
    EXPLORE_EXIT,
    EXPORT_CVN_EXIT,
    EXTERNAL_ARTIFACT_SCHEMA,
    LOWER_EXIT,
    REPAIR_EXIT,
    REPLAY_EXIT,
    SEARCH_ARTIFACT_SCHEMA,
    SUMMARIZE_EXIT,
    SUPPORT_EXIT,
    VALIDATE_CONTRACT_EXIT,
    VERIFY_DESIGN_EXIT,
    VERIFY_REFINEMENT_EXIT,
    ConcirIdentity,
    ConcirResult,
    _classify,
)
from .runner import ConcirSubprocessRunner

__all__ = [
    "CHECK_EXIT",
    "CHECK_REQ_EXIT",
    "EXPLAIN_CONTRACT_EXIT",
    "EXPLORE_EXIT",
    "EXPORT_CVN_EXIT",
    "EXTERNAL_ARTIFACT_SCHEMA",
    "LOWER_EXIT",
    "REPAIR_EXIT",
    "REPLAY_EXIT",
    "SEARCH_ARTIFACT_SCHEMA",
    "SUMMARIZE_EXIT",
    "SUPPORT_EXIT",
    "VALIDATE_CONTRACT_EXIT",
    "VERIFY_DESIGN_EXIT",
    "VERIFY_REFINEMENT_EXIT",
    "ConcirClient",
    "ConcirIdentity",
    "ConcirResult",
    "ConcirSubprocessRunner",
    "_classify",
]
