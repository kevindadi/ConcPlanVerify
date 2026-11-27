"""Zero-dependency Model Context Protocol (MCP) JSON-RPC 2.0 stdio server core.

Implements standard MCP lifecycle, tools, resources, and prompts over ``stdin``/``stdout``
using Python's standard library only. Supports both newline-delimited JSON-RPC
(standard MCP stdio transport) and ``Content-Length:`` framed messages on input,
and always writes clean newline-delimited UTF-8 JSON to ``stdout``.
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from typing import Any, BinaryIO, Callable, TextIO

SUPPORTED_PROTOCOL_VERSIONS: tuple[str, ...] = ("2025-03-26", "2024-11-05")
DEFAULT_PROTOCOL_VERSION = "2025-03-26"

PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603


@dataclass
class ToolSpec:
    """Declarative specification for one MCP tool."""

    name: str
    description: str
    input_schema: dict[str, Any]
    handler: Callable[[dict[str, Any]], dict[str, Any]]

    def to_mcp(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        }


@dataclass
class ResourceSpec:
    """Declarative specification for one read-only MCP resource."""

    uri: str
    name: str
    description: str
    mime_type: str
    reader: Callable[[], str]

    def to_mcp(self) -> dict[str, Any]:
        return {
            "uri": self.uri,
            "name": self.name,
            "description": self.description,
            "mimeType": self.mime_type,
        }


@dataclass
class PromptArgument:
    """One argument accepted by an MCP prompt template."""

    name: str
    description: str
    required: bool = False

    def to_mcp(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "required": self.required,
        }


@dataclass
class PromptSpec:
    """Declarative specification for one MCP prompt template."""

    name: str
    description: str
    arguments: list[PromptArgument] = field(default_factory=list)
    renderer: Callable[[dict[str, str]], list[dict[str, Any]]] | None = None

    def to_mcp(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "arguments": [a.to_mcp() for a in self.arguments],
        }


def _jsonrpc_result(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def _jsonrpc_error(
    req_id: Any,
    code: int,
    message: str,
    data: Any = None,
) -> dict[str, Any]:
    err: dict[str, Any] = {"code": code, "message": message}
    if data is not None:
        err["data"] = data
    return {"jsonrpc": "2.0", "id": req_id, "error": err}


class McpServer:
    """In-process and stdio JSON-RPC 2.0 MCP server."""

    def __init__(
        self,
        *,
        name: str = "concplan-mcp",
        version: str = "0.5.0",
        instructions: str | None = None,
    ) -> None:
        self.name = name
        self.version = version
        self.instructions = instructions
        self._tools: dict[str, ToolSpec] = {}
        self._resources: dict[str, ResourceSpec] = {}
        self._prompts: dict[str, PromptSpec] = {}

    @property
    def tools(self) -> dict[str, ToolSpec]:
        return dict(self._tools)

    @property
    def resources(self) -> dict[str, ResourceSpec]:
        return dict(self._resources)

    @property
    def prompts(self) -> dict[str, PromptSpec]:
        return dict(self._prompts)

    def register_tool(self, spec: ToolSpec) -> None:
        self._tools[spec.name] = spec

    def register_resource(self, spec: ResourceSpec) -> None:
        self._resources[spec.uri] = spec

    def register_prompt(self, spec: PromptSpec) -> None:
        self._prompts[spec.name] = spec

    def describe(self) -> dict[str, Any]:
        return {
            "server": {"name": self.name, "version": self.version},
            "tools": [t.to_mcp() for t in self._tools.values()],
            "resources": [r.to_mcp() for r in self._resources.values()],
            "prompts": [p.to_mcp() for p in self._prompts.values()],
        }

    def handle_message(self, msg: Any) -> dict[str, Any] | None:
        """Dispatch a single parsed JSON-RPC message.

        Returns ``None`` for notifications (messages without an ``"id"`` field).
        """
        if not isinstance(msg, dict):
            return _jsonrpc_error(None, INVALID_REQUEST, "JSON-RPC message must be an object")

        has_id = "id" in msg
        req_id = msg.get("id")
        method = msg.get("method")

        if not isinstance(method, str) or not method:
            if not has_id:
                return None
            return _jsonrpc_error(req_id, INVALID_REQUEST, "missing or invalid 'method'")

        # Notifications have no `id` and expect no response.
        if not has_id or method.startswith("notifications/"):
            if not has_id:
                return None

        params = msg.get("params")
        if params is None:
            params = {}
        elif not isinstance(params, dict):
            return _jsonrpc_error(req_id, INVALID_PARAMS, "'params' must be an object")

        if method == "initialize":
            return _jsonrpc_result(req_id, self._handle_initialize(params))
        if method == "ping":
            return _jsonrpc_result(req_id, {})
        if method == "tools/list":
            return _jsonrpc_result(
                req_id,
                {"tools": [t.to_mcp() for t in self._tools.values()]},
            )
        if method == "tools/call":
            return self._handle_tool_call(req_id, params)
        if method == "resources/list":
            return _jsonrpc_result(
                req_id,
                {"resources": [r.to_mcp() for r in self._resources.values()]},
            )
        if method == "resources/read":
            return self._handle_resource_read(req_id, params)
        if method == "prompts/list":
            return _jsonrpc_result(
                req_id,
                {"prompts": [p.to_mcp() for p in self._prompts.values()]},
            )
        if method == "prompts/get":
            return self._handle_prompt_get(req_id, params)

        return _jsonrpc_error(req_id, METHOD_NOT_FOUND, f"unknown method '{method}'")

    def _handle_initialize(self, params: dict[str, Any]) -> dict[str, Any]:
        requested = str(params.get("protocolVersion") or "")
        protocol_version = (
            requested if requested in SUPPORTED_PROTOCOL_VERSIONS else DEFAULT_PROTOCOL_VERSION
        )
        capabilities: dict[str, Any] = {
            "tools": {"listChanged": False},
            "resources": {"subscribe": False, "listChanged": False},
            "prompts": {"listChanged": False},
        }
        result: dict[str, Any] = {
            "protocolVersion": protocol_version,
            "capabilities": capabilities,
            "serverInfo": {"name": self.name, "version": self.version},
        }
        if self.instructions:
            result["instructions"] = self.instructions
        return result

    def _handle_tool_call(self, req_id: Any, params: dict[str, Any]) -> dict[str, Any]:
        name = params.get("name")
        if not isinstance(name, str) or not name:
            return _jsonrpc_error(req_id, INVALID_PARAMS, "tools/call requires string 'name'")
        tool = self._tools.get(name)
        if tool is None:
            return _jsonrpc_error(req_id, INVALID_PARAMS, f"unknown tool '{name}'")

        raw_args = params.get("arguments")
        if raw_args is None:
            arguments: dict[str, Any] = {}
        elif isinstance(raw_args, dict):
            arguments = raw_args
        else:
            return _jsonrpc_error(req_id, INVALID_PARAMS, "tools/call 'arguments' must be an object")

        try:
            payload = tool.handler(arguments)
        except Exception as exc:  # noqa: BLE001
            err_obj = {"error": str(exc), "error_type": type(exc).__name__}
            return _jsonrpc_result(
                req_id,
                {
                    "content": [{"type": "text", "text": json.dumps(err_obj, ensure_ascii=False)}],
                    "isError": True,
                },
            )

        if not isinstance(payload, dict):
            payload = {"result": payload}
        return _jsonrpc_result(
            req_id,
            {
                "content": [
                    {"type": "text", "text": json.dumps(payload, ensure_ascii=False, indent=2)}
                ],
                "structuredContent": payload,
                "isError": False,
            },
        )

    def _handle_resource_read(self, req_id: Any, params: dict[str, Any]) -> dict[str, Any]:
        uri = params.get("uri")
        if not isinstance(uri, str) or not uri:
            return _jsonrpc_error(req_id, INVALID_PARAMS, "resources/read requires string 'uri'")
        res = self._resources.get(uri)
        if res is None:
            return _jsonrpc_error(req_id, INVALID_PARAMS, f"unknown resource '{uri}'")
        try:
            text = res.reader()
        except Exception as exc:  # noqa: BLE001
            return _jsonrpc_error(req_id, INTERNAL_ERROR, f"failed to read '{uri}': {exc}")
        return _jsonrpc_result(
            req_id,
            {
                "contents": [
                    {
                        "uri": res.uri,
                        "mimeType": res.mime_type,
                        "text": text,
                    }
                ]
            },
        )

    def _handle_prompt_get(self, req_id: Any, params: dict[str, Any]) -> dict[str, Any]:
        name = params.get("name")
        if not isinstance(name, str) or not name:
            return _jsonrpc_error(req_id, INVALID_PARAMS, "prompts/get requires string 'name'")
        prompt = self._prompts.get(name)
        if prompt is None:
            return _jsonrpc_error(req_id, INVALID_PARAMS, f"unknown prompt '{name}'")

        raw_args = params.get("arguments")
        if raw_args is None:
            args: dict[str, str] = {}
        elif isinstance(raw_args, dict):
            args = {str(k): str(v) for k, v in raw_args.items() if v is not None}
        else:
            return _jsonrpc_error(req_id, INVALID_PARAMS, "prompts/get 'arguments' must be an object")

        missing = [a.name for a in prompt.arguments if a.required and not args.get(a.name)]
        if missing:
            return _jsonrpc_error(
                req_id, INVALID_PARAMS, f"missing required prompt argument(s): {missing}"
            )

        messages = prompt.renderer(args) if prompt.renderer is not None else []
        return _jsonrpc_result(
            req_id,
            {
                "description": prompt.description,
                "messages": messages,
            },
        )

    def serve_stdio(
        self,
        stdin: BinaryIO | TextIO | None = None,
        stdout: BinaryIO | TextIO | None = None,
    ) -> None:
        """Serve MCP JSON-RPC 2.0 requests from ``stdin`` and write responses to ``stdout``.

        Accepts both newline-delimited JSON-RPC lines and ``Content-Length:`` framed
        messages on ``stdin``. Writes newline-delimited UTF-8 JSON to ``stdout``.
        """
        in_stream = stdin if stdin is not None else sys.stdin.buffer
        out_stream = stdout if stdout is not None else sys.stdout.buffer

        while True:
            raw_line = in_stream.readline()
            if not raw_line:
                break
            line_str = (
                raw_line.decode("utf-8", "replace")
                if isinstance(raw_line, (bytes, bytearray))
                else raw_line
            )
            stripped = line_str.strip()
            if not stripped:
                continue

            payload_text: str | None = None
            if stripped.lower().startswith("content-length:"):
                try:
                    length = int(stripped.split(":", 1)[1].strip())
                except ValueError:
                    self._write_message(
                        out_stream,
                        _jsonrpc_error(None, PARSE_ERROR, f"invalid Content-Length header: {stripped}"),
                    )
                    continue
                # Consume remaining headers up to the blank line.
                while True:
                    hdr = in_stream.readline()
                    if not hdr:
                        return
                    hdr_str = (
                        hdr.decode("utf-8", "replace")
                        if isinstance(hdr, (bytes, bytearray))
                        else hdr
                    )
                    if not hdr_str.strip():
                        break
                body = in_stream.read(length)
                if not body:
                    break
                payload_text = (
                    body.decode("utf-8", "replace")
                    if isinstance(body, (bytes, bytearray))
                    else body
                )
            else:
                payload_text = stripped

            try:
                msg = json.loads(payload_text)
            except json.JSONDecodeError as exc:
                self._write_message(
                    out_stream,
                    _jsonrpc_error(None, PARSE_ERROR, f"invalid JSON: {exc}"),
                )
                continue

            if isinstance(msg, list):
                responses = [
                    resp for item in msg if (resp := self.handle_message(item)) is not None
                ]
                if responses:
                    self._write_message(out_stream, responses)
            else:
                resp = self.handle_message(msg)
                if resp is not None:
                    self._write_message(out_stream, resp)

    @staticmethod
    def _write_message(out_stream: BinaryIO | TextIO, payload: Any) -> None:
        encoded = json.dumps(payload, ensure_ascii=False) + "\n"
        try:
            out_stream.write(encoded.encode("utf-8"))
        except TypeError:
            out_stream.write(encoded)  # type: ignore[arg-type]
        out_stream.flush()
