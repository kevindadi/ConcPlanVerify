"""Thin compatibility facade re-exporting ``concplan.core.client``."""

from .client import (
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
    ConcirClient,
    ConcirIdentity,
    ConcirResult,
    _classify,
)
from .client.protocol import validate_replay_payload as _validate_replay_payload
from .json_utils import sha256_file

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
    "_classify",
    "_validate_replay_payload",
    "sha256_file",
]
