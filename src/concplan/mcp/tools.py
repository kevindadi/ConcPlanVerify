"""ConcPlanVerify MCP tool registration facade."""

from __future__ import annotations

from .handlers_code import build_code_tool_specs
from .handlers_model import build_model_tool_specs
from .protocol import McpServer
from .workspace import McpWorkspace


def register_concplan_tools(server: McpServer, ws: McpWorkspace) -> None:
    """Register all ConcPlanVerify Stage 0-4 MCP tools on ``server``."""
    for spec in [*build_model_tool_specs(ws), *build_code_tool_specs(ws)]:
        server.register_tool(spec)


__all__ = ["McpWorkspace", "register_concplan_tools"]
