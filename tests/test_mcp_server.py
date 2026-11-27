"""Tests for the zero-dependency ConcPlanVerify MCP server (`concplan.mcp`)."""

from __future__ import annotations

import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from concplan.core.toolchain import REPO, find_concir_backend, find_spn
from concplan.mcp import (
    DEFAULT_PROTOCOL_VERSION,
    McpServer,
    create_server,
)


def _call_tool(server: McpServer, name: str, arguments: dict | None = None, *, req_id: int = 1) -> tuple[dict, dict]:
    resp = server.handle_message({
        "jsonrpc": "2.0",
        "id": req_id,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments or {}},
    })
    assert resp is not None
    result = resp["result"]
    payload = result.get("structuredContent")
    if payload is None:
        payload = json.loads(result["content"][0]["text"])
    return result, payload


class McpProtocolTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.server = create_server(workdir=self.root / "mcp-work")

    def tearDown(self):
        self._tmp.cleanup()

    def test_lifecycle_lists_and_error_codes(self):
        init = self.server.handle_message({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "clientInfo": {"name": "test"}},
        })
        self.assertEqual(init["result"]["protocolVersion"], "2024-11-05")
        self.assertEqual(init["result"]["serverInfo"]["name"], "concplan-mcp")
        self.assertIn("tools", init["result"]["capabilities"])
        self.assertIn("resources", init["result"]["capabilities"])
        self.assertIn("prompts", init["result"]["capabilities"])

        init_fallback = self.server.handle_message({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {"protocolVersion": "1999-01-01"},
        })
        self.assertEqual(init_fallback["result"]["protocolVersion"], DEFAULT_PROTOCOL_VERSION)

        self.assertIsNone(
            self.server.handle_message({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        self.assertEqual(
            self.server.handle_message({"jsonrpc": "2.0", "id": 3, "method": "ping"})["result"],
            {},
        )

        tools = self.server.handle_message({"jsonrpc": "2.0", "id": 4, "method": "tools/list"})["result"]["tools"]
        tool_names = {t["name"] for t in tools}
        self.assertEqual(len(tool_names), 12)
        self.assertIn("concplan_explore_model", tool_names)
        self.assertIn("concplan_verify_code", tool_names)

        resources = self.server.handle_message({"jsonrpc": "2.0", "id": 5, "method": "resources/list"})["result"]["resources"]
        uris = {r["uri"] for r in resources}
        self.assertIn("concplan://docs/agents", uris)
        self.assertIn("concplan://prompts/scir-generation", uris)

        agents_res = self.server.handle_message({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "resources/read",
            "params": {"uri": "concplan://docs/agents"},
        })["result"]["contents"][0]
        self.assertIn("ConcPlanVerify", agents_res["text"])

        prompts = self.server.handle_message({"jsonrpc": "2.0", "id": 7, "method": "prompts/list"})["result"]["prompts"]
        self.assertEqual(
            {p["name"] for p in prompts},
            {"design_concurrent_rust", "formalize_requirements", "repair_hole_violations"},
        )
        prompt_msg = self.server.handle_message({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "prompts/get",
            "params": {"name": "design_concurrent_rust", "arguments": {"requirements": "Protect c with m."}},
        })["result"]["messages"][0]
        self.assertIn("Protect c with m.", prompt_msg["content"]["text"])

        unknown = self.server.handle_message({"jsonrpc": "2.0", "id": 9, "method": "nope/method"})
        self.assertEqual(unknown["error"]["code"], -32601)

    def test_stdio_newline_and_content_length_framing(self):
        msg1 = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "ping"})
        msg2_bytes = json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}).encode("utf-8")
        framed2 = f"Content-Length: {len(msg2_bytes)}\r\n\r\n".encode("ascii") + msg2_bytes
        raw_input = (msg1 + "\n").encode("utf-8") + b"{bad json\n" + framed2

        in_buf = io.BytesIO(raw_input)
        out_buf = io.BytesIO()
        self.server.serve_stdio(stdin=in_buf, stdout=out_buf)

        lines = [json.loads(line) for line in out_buf.getvalue().decode("utf-8").splitlines() if line.strip()]
        self.assertEqual(len(lines), 3)
        self.assertEqual(lines[0], {"jsonrpc": "2.0", "id": 1, "result": {}})
        self.assertEqual(lines[1]["error"]["code"], -32700)
        self.assertEqual(lines[2]["id"], 2)
        self.assertEqual(len(lines[2]["result"]["tools"]), 12)


class McpOfflineToolsTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.server = create_server(workdir=self.root / "mcp-work")

    def tearDown(self):
        self._tmp.cleanup()

    def test_toolchain_status_and_list_examples(self):
        res_tc, payload_tc = _call_tool(self.server, "concplan_toolchain_status")
        self.assertFalse(res_tc["isError"])
        self.assertIn("concir_backend", payload_tc)
        self.assertIn("spn", payload_tc)

        res_ex, payload_ex = _call_tool(self.server, "concplan_list_examples")
        self.assertFalse(res_ex["isError"])
        self.assertGreaterEqual(payload_ex["total"], 4)

        res_one, payload_one = _call_tool(
            self.server,
            "concplan_list_examples",
            {"example_id": "shared_counter"},
        )
        self.assertFalse(res_one["isError"])
        self.assertIsNotNone(payload_one["spec_md"])
        self.assertIn("requirements", payload_one)
        self.assertIsNotNone(payload_one["contract"])

    def test_validate_contract_grounding_and_non_vacuity(self):
        reqs = (
            "## Entities\n"
            "- Roles: `w1`, `w2`\n"
            "- Resources: `m`, `c`\n\n"
            "## Clauses\n"
            "- R1 [V] (safety): No deadlock.\n"
            "- R2 [V] (liveness): When c > 0, w1 eventually completes.\n"
            "- R3 [U] (sequential_hole): Format output string.\n"
        )
        contract_imply = {
            "name": "imply-test",
            "properties": [
                {"kind": "deadlock_free", "id": "p_dl", "req": ["R1"]},
                {
                    "kind": "ctl",
                    "id": "p_resp",
                    "req": ["R2"],
                    "formula": {
                        "op": "ag",
                        "arg": {
                            "op": "implies",
                            "lhs": {"op": "atom", "atom": {"kind": "var_cmp", "resource": "main::c", "op": ">", "value": 0}},
                            "rhs": {"op": "af", "arg": {"op": "atom", "atom": {"kind": "function_completed", "function": "main::w1"}}},
                        },
                    },
                },
            ],
        }
        res_ok, payload_ok = _call_tool(
            self.server,
            "concplan_validate_contract",
            {"contract": contract_imply, "requirements": reqs},
        )
        self.assertFalse(res_ok["isError"])
        self.assertTrue(payload_ok["ok"], payload_ok)
        self.assertTrue(payload_ok["contract"]["properties"][1]["non_vacuous"])
        self.assertIn("explanation", payload_ok)

        bad_contract = {
            "name": "bad-grounding",
            "properties": [
                {
                    "kind": "safety",
                    "id": "p_bad",
                    "invariant": {"kind": "mutex_free", "resource": "ghost_lock"},
                    "req": ["R3"],
                }
            ],
        }
        res_bad, payload_bad = _call_tool(
            self.server,
            "concplan_validate_contract",
            {"contract": bad_contract, "requirements": reqs},
        )
        self.assertFalse(res_bad["isError"])
        self.assertFalse(payload_bad["ok"])
        codes = {i["code"] for i in payload_bad["sanity"]["issues"]}
        self.assertIn("unverifiable_req_clause", codes)
        self.assertIn("ungrounded_resource", codes)

    def test_fill_holes_and_verify_code_ast_only_catches_seq_and_type_hole_violations(self):
        skel = self.root / "skel"
        (skel / "src").mkdir(parents=True)
        (skel / "src/main.rs").write_text(
            "mod domain;\n"
            "fn main() {\n"
            "    // @cir s1\n"
            "    let _x: i64 = /* HOLE(h1) expected: i64 */ Default::default();\n"
            "}\n",
            encoding="utf-8",
        )
        (skel / "src/domain.rs").write_text(
            "/* TYPE_HOLE(th1) type: OrderBook */\n"
            "#[derive(Clone, Debug, Default, PartialEq)]\n"
            "pub struct OrderBook {}\n",
            encoding="utf-8",
        )
        (skel / "codegen.json").write_text(
            json.dumps({
                "holes": [{"id": "h1", "module": "main", "function": "main", "sid": "s1", "expected": "i64", "file": "src/main.rs"}],
                "type_holes": [{"id": "th1", "type_name": "OrderBook", "module": "main", "file": "src/domain.rs", "line": 1}],
            }),
            encoding="utf-8",
        )

        # Pure fills pass.
        res_fill, fill_payload = _call_tool(
            self.server,
            "concplan_fill_holes",
            {
                "skeleton_dir": str(skel),
                "filled_dir": str(self.root / "filled_ok"),
                "fills": {"h1": "42"},
                "type_fills": {"th1": "#[derive(Clone, Debug, Default, PartialEq)]\npub struct OrderBook { pub bid: i64 }"},
            },
        )
        self.assertFalse(res_fill["isError"])
        self.assertEqual(fill_payload["applied"], ["h1"])
        self.assertEqual(fill_payload["applied_type_holes"], ["th1"])

        res_v, v_payload = _call_tool(
            self.server,
            "concplan_verify_code",
            {"skeleton_dir": str(skel), "filled_dir": fill_payload["filled_dir"], "ast_only": True},
        )
        self.assertFalse(res_v["isError"])
        self.assertEqual(v_payload["status"], "pass")

        # Forbidden Mutex in TypeHole fails with offending_holes = ["th1"].
        res_bad_fill, bad_fill_payload = _call_tool(
            self.server,
            "concplan_fill_holes",
            {
                "skeleton_dir": str(skel),
                "filled_dir": str(self.root / "filled_bad"),
                "fills": {
                    "h1": "42",
                    "th1": "pub struct OrderBook { pub m: std::sync::Mutex<i64> }",
                },
            },
        )
        self.assertFalse(res_bad_fill["isError"])
        res_bad_v, bad_v_payload = _call_tool(
            self.server,
            "concplan_verify_code",
            {"skeleton_dir": str(skel), "filled_dir": bad_fill_payload["filled_dir"], "ast_only": True},
        )
        self.assertFalse(res_bad_v["isError"])
        self.assertEqual(bad_v_payload["status"], "fail_hole")
        self.assertEqual(bad_v_payload["offending_holes"], ["th1"])

    def test_tool_error_returns_is_error_instead_of_crashing(self):
        res, payload = _call_tool(self.server, "concplan_validate_contract", {})
        self.assertTrue(res["isError"])
        self.assertIn("error", payload)

    def test_cli_list_tools_entrypoint(self):
        proc = subprocess.run(
            [sys.executable, "-m", "concplan.mcp", "--list-tools"],
            cwd=str(REPO),
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(proc.returncode, 0, proc.stderr)
        data = json.loads(proc.stdout)
        self.assertEqual(len(data["tools"]), 12)


class McpLiveWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.backend = find_concir_backend()
        self.spn = find_spn()
        if self.backend is None:
            self.skipTest("concir-backend binary not built")
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.server = create_server(workdir=self.root / "mcp-work", concir_binary=self.backend, spn_binary=self.spn)

    def tearDown(self):
        self._tmp.cleanup()

    def test_live_mcp_end_to_end_concurrent_rust_workflow(self):
        scir_prog = {
            "program": "mcp_counter",
            "version": "5.0.0",
            "entry": "main::main",
            "modules": [
                {
                    "name": "main",
                    "req": ["R1", "R2"],
                    "provides": {"resources": ["m", "c"], "functions": ["main", "w1", "w2"]},
                    "requires": {"resources": [], "functions": []},
                    "resources": [
                        {"name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync", "req": ["R1"]},
                        {"name": "c", "kind": "var", "type": "Var", "base": {"Int": [0, 2]}, "init": 0, "req": ["R2"]},
                    ],
                    "protection": [{"var": "c", "lock": "m"}],
                    "functions": [
                        {
                            "name": "main",
                            "kind": "normal",
                            "structured_body": [
                                {"id": "b_scope", "kind": "scope", "funcs": ["main::w1", "main::w2"], "req": ["R1"]},
                                {"id": "b_ret", "kind": "return"},
                            ],
                        },
                        {
                            "name": "w1",
                            "kind": "normal",
                            "form": "closure",
                            "structured_body": [
                                {
                                    "id": "b_lg1",
                                    "kind": "lock_guard",
                                    "resource": "main::m",
                                    "body": [
                                        {
                                            "id": "h1",
                                            "kind": "seq_hole",
                                            "reads": ["c"],
                                            "writes": ["c"],
                                            "updates_modeled": [{"target": "c", "expr": "c + 1"}],
                                            "expected_type": "i64",
                                            "req": ["R2"],
                                        }
                                    ],
                                },
                                {"id": "b_r1", "kind": "return"},
                            ],
                        },
                        {
                            "name": "w2",
                            "kind": "normal",
                            "form": "closure",
                            "structured_body": [
                                {
                                    "id": "b_lg2",
                                    "kind": "lock_guard",
                                    "resource": "main::m",
                                    "body": [
                                        {
                                            "id": "h2",
                                            "kind": "seq_hole",
                                            "reads": ["c"],
                                            "writes": ["c"],
                                            "updates_modeled": [{"target": "c", "expr": "c + 1"}],
                                            "expected_type": "i64",
                                            "req": ["R2"],
                                        }
                                    ],
                                },
                                {"id": "b_r2", "kind": "return"},
                            ],
                        },
                    ],
                }
            ],
        }
        contract = {
            "name": "mcp-counter-contract",
            "properties": [
                {"kind": "deadlock_free", "id": "no-deadlock", "req": ["R1"]},
                {
                    "kind": "safety",
                    "id": "bound",
                    "invariant": {"kind": "var_cmp", "resource": "main::c", "op": "<=", "value": 2},
                    "req": ["R2"],
                },
            ],
            "preserved": [
                {"kind": "reachable", "description": "w1 done", "goal": {"kind": "function_completed", "function": "main::w1"}},
                {"kind": "reachable", "description": "w2 done", "goal": {"kind": "function_completed", "function": "main::w2"}},
            ],
        }

        _, low = _call_tool(self.server, "concplan_lower_scir", {"program": scir_prog})
        self.assertEqual(low["status"], "pass")
        self.assertIsNotNone(low["trace_map"])

        _, chk = _call_tool(self.server, "concplan_check_model", {"program": low["lowered_program"]})
        self.assertTrue(chk["valid"])
        self.assertTrue(chk["supported"])

        _, exp = _call_tool(
            self.server,
            "concplan_explore_model",
            {"program": low["lowered_program"], "contract": contract, "engine": "petri"},
        )
        self.assertEqual(exp["outcome"], "PASS", exp)
        self.assertTrue(exp["complete"])

        _, cg = _call_tool(
            self.server,
            "concplan_codegen_skeleton",
            {"program": low["lowered_program"], "requirements": "R1: lock m. R2: increment c."},
        )
        self.assertEqual({h["id"] for h in cg["holes"]}, {"h1", "h2"})
        self.assertIn("fill_prompt", cg)

        _, filled = _call_tool(
            self.server,
            "concplan_fill_holes",
            {"skeleton_dir": cg["skeleton_dir"], "fills": {"h1": "1", "h2": "2"}},
        )
        self.assertEqual(set(filled["applied"]), {"h1", "h2"})

        _, verified = _call_tool(
            self.server,
            "concplan_verify_code",
            {
                "skeleton_dir": cg["skeleton_dir"],
                "filled_dir": filled["filled_dir"],
                "program": low["lowered_program"],
                "contract": contract,
                "ast_only": self.spn is None,
            },
        )
        self.assertEqual(verified["status"], "pass", verified)
        self.assertTrue(verified["ok"])


if __name__ == "__main__":
    unittest.main()
