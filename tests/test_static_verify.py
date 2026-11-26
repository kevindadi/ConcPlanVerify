"""Unit tests for toolchain discovery, ConcirClient extensions, and static_verify plugins."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from concplan.code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    LockbudPlugin,
    SpnMirRefinementPlugin,
    augment_codegen_map_with_hole_ranges,
    fill_holes_marked,
    strip_rust_comments_and_literals,
)
from concplan.core import toolchain
from concplan.core.concir_client import ConcirClient
from tests._helpers import make_stub_binary, stub_env

SAMPLE_SKELETON = """\
fn cf_w1() {
    cir_trace::ev(tag, "s1"); // @cir s1
    let _hole_val: i64 = /* HOLE(h1) expected: i64 */ Default::default();
    cir_trace::ev(tag, "s2"); // @cir s2
    let _hole_val2: () = /* HOLE(h2) expected: () */ Default::default();
    cir_trace::ev(tag, "s3"); // @cir s3
}
"""

SAMPLE_CODEGEN = {
    "resources": [
        {"fqn": "main::m", "field": "r0_main_m", "kind": "Mutex"},
        {"fqn": "main::c", "field": "r1_main_c", "kind": "Var"},
    ],
    "holes": [
        {
            "id": "h1",
            "module": "main",
            "function": "main::w1",
            "sid": "s1",
            "expected": "i64",
            "line": 3,
            "reads": ["main::c"],
            "writes": [],
            "req": ["R1"],
        },
        {
            "id": "h2",
            "module": "main",
            "function": "main::w1",
            "sid": "s2",
            "expected": "()",
            "line": 5,
            "reads": ["main::c"],
            "writes": ["main::c"],
            "req": ["R2"],
        },
    ],
}


def _make_skeleton(root: Path, source: str = SAMPLE_SKELETON, codegen: dict | None = None) -> Path:
    skel = root / "skeleton"
    (skel / "src").mkdir(parents=True, exist_ok=True)
    (skel / "Cargo.toml").write_text(
        '[package]\nname = "cir_generated"\nversion = "0.1.0"\nedition = "2021"\n'
        '[features]\ndefault = ["cir_trace"]\ncir_trace = []\n',
        encoding="utf-8",
    )
    (skel / "src/main.rs").write_text(source, encoding="utf-8")
    (skel / "codegen.json").write_text(
        json.dumps(codegen or SAMPLE_CODEGEN, indent=2), encoding="utf-8"
    )
    return skel


class ToolchainSubmoduleResolutionTests(unittest.TestCase):
    def test_submodule_checked_before_legacy_and_exe_handled(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sub_rel = root / "sub_release"
            sub_dbg = root / "sub_debug"
            leg_rel = root / "leg_release"
            for d in (sub_rel, sub_dbg, leg_rel):
                d.mkdir(parents=True, exist_ok=True)

            exe_suffix = ".exe" if os.name == "nt" else ""
            sub_bin = sub_rel / f"concir-backend{exe_suffix}"
            leg_bin = leg_rel / f"concir-backend{exe_suffix}"
            sub_bin.write_text("sub", encoding="utf-8")
            leg_bin.write_text("leg", encoding="utf-8")

            saved = {k: os.environ.pop(k, None) for k in ("CONCIR_BACKEND", "CONCIR_TOOLCHAIN")}
            try:
                with mock.patch.object(
                    toolchain, "CONCIR_SEARCH_DIRS", (sub_rel, sub_dbg, leg_rel)
                ):
                    found = toolchain.find_concir_backend()
                    self.assertEqual(found, sub_bin.resolve())

                # Removing submodule release falls back to legacy.
                sub_bin.unlink()
                with mock.patch.object(
                    toolchain, "CONCIR_SEARCH_DIRS", (sub_rel, sub_dbg, leg_rel)
                ):
                    found = toolchain.find_concir_backend()
                    self.assertEqual(found, leg_bin.resolve())
            finally:
                for k, v in saved.items():
                    if v is not None:
                        os.environ[k] = v

    def test_find_spn_honors_env_and_exe(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            exe_suffix = ".exe" if os.name == "nt" else ""
            fake_spn = root / f"spn{exe_suffix}"
            fake_spn.write_text("spn", encoding="utf-8")
            # Pass path WITHOUT .exe suffix in env on Windows to verify seamless resolution.
            env_path = str(root / "spn")
            with mock.patch.dict(os.environ, {"CONCIR_SPN": env_path}):
                found = toolchain.find_spn()
                self.assertEqual(found, fake_spn.resolve())


class ConcirClientV4MethodsTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.stub = make_stub_binary(self.root / "bin")
        self.prog = self.root / "prog.json"
        self.prog.write_text('{"program": "demo", "modules": []}', encoding="utf-8")
        self.impl_net = self.root / "impl_net.json"
        self.impl_net.write_text('{"schema": "concir-impl-net/1", "states": []}', encoding="utf-8")

    def tearDown(self):
        self._tmp.cleanup()

    def _client(self, **env) -> ConcirClient:
        return ConcirClient(
            self.stub, workdir=self.root / "work", timeout=5.0, env=stub_env(**env)
        )

    def test_lower_pass_and_invalid(self):
        out_p = self.root / "out" / "lowered.json"
        tm_p = self.root / "out" / "trace_map.json"
        r = self._client(
            STUB_STDOUT='{"program": "demo", "modules": []}', STUB_EXIT="0"
        ).lower(self.prog, out_path=out_p, trace_map_path=tm_p)
        self.assertEqual((r.kind, r.status), ("semantic", "pass"))
        self.assertIn("--out", r.argv)
        self.assertIn("--trace-map", r.argv)

        r_inv = self._client(
            STUB_STDOUT='{"valid": false, "diagnostics": []}', STUB_EXIT="4"
        ).lower(self.prog)
        self.assertEqual((r_inv.kind, r_inv.status), ("semantic", "invalid"))

    def test_summarize_acyclic_and_cycle_and_contradiction(self):
        ok_payload = json.dumps({
            "schema": "concir-module-summary/1",
            "cross_module_lock_cycle": False,
            "modules": [{"name": "main", "cross_module_lock_cycle": False}],
        })
        r_ok = self._client(STUB_STDOUT=ok_payload, STUB_EXIT="0").summarize(
            self.prog, module="main"
        )
        self.assertEqual((r_ok.kind, r_ok.status), ("semantic", "pass"))
        self.assertIn("--module", r_ok.argv)

        cyc_payload = json.dumps({
            "schema": "concir-module-summary/1",
            "cross_module_lock_cycle": True,
            "modules": [{"name": "main", "cross_module_lock_cycle": True}],
        })
        r_fail = self._client(STUB_STDOUT=cyc_payload, STUB_EXIT="1").summarize(self.prog)
        self.assertEqual((r_fail.kind, r_fail.status), ("semantic", "fail"))

        # Contradictory exit 0 with cycle=true must be a protocol_error.
        r_bad = self._client(STUB_STDOUT=cyc_payload, STUB_EXIT="0").summarize(self.prog)
        self.assertEqual(r_bad.kind, "protocol_error")

    def test_verify_refinement_exit_codes(self):
        for outcome, code in (
            ("pass", "0"),
            ("fail", "1"),
            ("unknown", "3"),
            ("invalid", "4"),
            ("unsupported", "5"),
        ):
            payload = json.dumps({
                "schema": "concir-refinement-report/1",
                "outcome": outcome,
                "violations": [],
            })
            r = self._client(STUB_STDOUT=payload, STUB_EXIT=code).verify_refinement(
                self.prog, self.impl_net
            )
            self.assertEqual((r.kind, r.status), ("semantic", outcome))

        # Outcome/exit mismatch -> protocol_error.
        r_mis = self._client(
            STUB_STDOUT='{"schema": "concir-refinement-report/1", "outcome": "fail"}',
            STUB_EXIT="0",
        ).verify_refinement(self.prog, self.impl_net)
        self.assertEqual(r_mis.kind, "protocol_error")

    def test_cli_subcommands_lower_summarize_verify_refinement(self):
        from concplan.__main__ import main as cli_main

        out_dir = str(self.root / "cli-work")
        with mock.patch.dict(
            os.environ,
            {"STUB_STDOUT": '{"program": "demo", "modules": []}', "STUB_EXIT": "0"},
        ):
            rc = cli_main([
                "--binary", str(self.stub), "--out", out_dir,
                "lower", str(self.prog),
            ])
            self.assertEqual(rc, 0)

        with mock.patch.dict(
            os.environ,
            {
                "STUB_STDOUT": '{"schema": "concir-module-summary/1", "cross_module_lock_cycle": false, "modules": []}',
                "STUB_EXIT": "0",
            },
        ):
            rc = cli_main([
                "--binary", str(self.stub), "--out", out_dir,
                "summarize", str(self.prog), "--module", "main",
            ])
            self.assertEqual(rc, 0)

        with mock.patch.dict(
            os.environ,
            {
                "STUB_STDOUT": '{"schema": "concir-refinement-report/1", "outcome": "pass", "violations": []}',
                "STUB_EXIT": "0",
            },
        ):
            rc = cli_main([
                "--binary", str(self.stub), "--out", out_dir,
                "verify-refinement", "--spec", str(self.prog), "--impl-net", str(self.impl_net),
            ])
            self.assertEqual(rc, 0)


class AstHolePurityAndHoleRangesTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.skel = _make_skeleton(self.root)

    def tearDown(self):
        self._tmp.cleanup()

    def test_comment_and_string_stripping_preserves_lines_and_lifetimes(self):
        code = (
            'let msg = "std::sync::Mutex x.lock() while loop unsafe";\n'
            'let raw = r#"thread::spawn(AtomicI64::new(0))"#;\n'
            "// Mutex::new(0) .wait() unsafe loop\n"
            "/* outer /* nested thread::spawn */ .lock() */\n"
            "let ch = '\\'';\n"
            "'a: loop { break 'a; }\n"
        )
        stripped = strip_rust_comments_and_literals(code)
        self.assertEqual(stripped.count("\n"), code.count("\n"))
        self.assertNotIn("Mutex", stripped)
        self.assertNotIn("thread::spawn", stripped)
        self.assertNotIn(".lock()", stripped)
        self.assertNotIn("unsafe", stripped)
        # Line 6 lifetime + real loop keyword must survive stripping!
        lines = stripped.splitlines()
        self.assertIn("loop", lines[5])

    def test_multiline_hole_ranges_marked_and_unmarked(self):
        fills = {
            "h1": "{\n        let a = 10;\n        let b = 20;\n        a + b\n    }",
            "h2": "{\n        // Mutex::new(0) in comment is safe!\n        let _s = \"std::thread::spawn\";\n        ()\n    }",
        }
        filled = self.root / "filled_marked"
        info = fill_holes_marked(self.skel, filled, fills)
        ranges = info["hole_ranges"]
        # h1 starts on line 3 and spans 5 lines (3..7)
        self.assertEqual(ranges["h1"], {"file": "src/main.rs", "start_line": 3, "end_line": 7})
        # h2 starts on line 9 and spans 5 lines (9..13)
        self.assertEqual(ranges["h2"], {"file": "src/main.rs", "start_line": 9, "end_line": 13})
        self.assertTrue(Path(info["codegen_filled_path"]).is_file())

        # Verify AstHolePurityPlugin passes on multi-line fills with forbidden words ONLY in comments/strings.
        plugin = AstHolePurityPlugin()
        rep = plugin.verify(skeleton_dir=self.skel, filled_dir=filled, fills=fills)
        self.assertEqual(rep.status, "pass")
        self.assertTrue(rep.checks["skeleton_integrity"])
        self.assertTrue(rep.checks["hole_purity"])

        # Also test unmarked multi-line fill via augment_codegen_map_with_hole_ranges.
        unmarked_src = SAMPLE_SKELETON.replace(
            "/* HOLE(h1) expected: i64 */ Default::default()", fills["h1"]
        ).replace(
            "/* HOLE(h2) expected: () */ Default::default()", fills["h2"]
        )
        out_unmarked = self.root / "codegen_unmarked.json"
        aug = augment_codegen_map_with_hole_ranges(
            self.skel, unmarked_src, fills, out_path=out_unmarked
        )
        self.assertEqual(aug["hole_ranges"]["h1"], {"file": "src/main.rs", "start_line": 3, "end_line": 7})
        self.assertEqual(aug["hole_ranges"]["h2"], {"file": "src/main.rs", "start_line": 9, "end_line": 13})

    def test_ast_purity_attributes_sync_unsafe_loop_to_exact_hole_and_line(self):
        fills = {
            "h1": "42",
            "h2": "{\n        let _x = 1;\n        m.lock().unwrap();\n        loop {}\n    }",
        }
        filled = self.root / "filled_bad"
        fill_holes_marked(self.skel, filled, fills)

        rep = AstHolePurityPlugin().verify(skeleton_dir=self.skel, filled_dir=filled, fills=fills)
        self.assertEqual(rep.status, "fail_hole")
        self.assertEqual(rep.offending_holes, ["h2"])
        kinds = {v.kind for v in rep.violations}
        self.assertIn("sync_in_hole", kinds)
        self.assertIn("loop_in_hole", kinds)

        sync_v = next(v for v in rep.violations if v.kind == "sync_in_hole")
        self.assertEqual(sync_v.hole_id, "h2")
        self.assertEqual(sync_v.sid, "s2")
        self.assertEqual(sync_v.req_clauses, ["R2"])
        # h2 starts on line 5; m.lock() is on relative line +2 -> line 7!
        self.assertTrue(sync_v.rust_span.startswith("src/main.rs:7:"))

        loop_v = next(v for v in rep.violations if v.kind == "loop_in_hole")
        self.assertTrue(loop_v.rust_span.startswith("src/main.rs:8:"))

    def test_change_outside_hole_fails_skeleton(self):
        filled = self.root / "filled_tampered"
        fill_holes_marked(self.skel, filled, {"h1": "1", "h2": "()"})
        main_rs = filled / "src/main.rs"
        main_rs.write_text(
            main_rs.read_text(encoding="utf-8").replace('cir_trace::ev(tag, "s3")', 'cir_trace::ev(tag, "sX")'),
            encoding="utf-8",
        )
        rep = AstHolePurityPlugin().verify(skeleton_dir=self.skel, filled_dir=filled)
        self.assertEqual(rep.status, "fail_skeleton")
        self.assertFalse(rep.checks["skeleton_integrity"])
        self.assertIn("changed_outside_hole", {v.kind for v in rep.violations})


class SpnMirRefinementAndLockbudPluginTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.skel = _make_skeleton(self.root)
        self.filled = self.root / "filled"
        self.fills = {"h1": "1 + 2", "h2": "{\n    let _a = 1;\n    ()\n}"}
        fill_holes_marked(self.skel, self.filled, self.fills)
        self.spec = self.root / "spec.json"
        self.spec.write_text('{"program": "demo", "modules": []}', encoding="utf-8")
        self.contract = self.root / "contract.json"
        self.contract.write_text('{"deadlock_free": true}', encoding="utf-8")
        self.stub = make_stub_binary(self.root / "bin")
        self.fake_spn = self.root / "bin" / ("spn.exe" if os.name == "nt" else "spn")
        self.fake_spn.write_text("fake spn", encoding="utf-8")

    def tearDown(self):
        self._tmp.cleanup()

    def test_spn_unavailable_returns_unknown_never_pass(self):
        plugin = SpnMirRefinementPlugin(spn_binary=None)
        with mock.patch("concplan.code_verify.static_verify.find_spn", return_value=None):
            rep = plugin.verify(
                skeleton_dir=self.skel,
                filled_dir=self.filled,
                spec_path=self.spec,
                contract_path=self.contract,
            )
        self.assertEqual(rep.status, "unknown")
        self.assertFalse(rep.ok)
        self.assertFalse(rep.checks["spn_available"])

    def _mock_cargo_exporting_net(self, net_payload: dict, returncode: int = 0):
        captured: dict = {}

        def _fake_run(argv, *, cwd=None, env=None, capture_output=True, text=True, timeout=None):
            captured["argv"] = list(argv)
            captured["env"] = dict(env or {})
            spn_flags = captured["env"].get("SPN_FLAGS", "")
            # Extract --export-concir-net path from SPN_FLAGS
            tokens = spn_flags.split()
            if "--export-concir-net" in tokens:
                idx = tokens.index("--export-concir-net")
                net_path = Path(tokens[idx + 1].strip('"'))
                net_path.parent.mkdir(parents=True, exist_ok=True)
                net_path.write_text(json.dumps(net_payload), encoding="utf-8")
            return subprocess.CompletedProcess(argv, returncode, stdout="", stderr="")

        return _fake_run, captured

    def test_spn_plugin_pass(self):
        ref_stdout = json.dumps({
            "schema": "concir-refinement-report/1",
            "outcome": "pass",
            "relation": "stuttering_refinement",
            "checks": {
                "hole_purity": True,
                "weak_simulation": True,
                "deadlock_free": True,
                "lock_divergence_free": True,
                "preserved_goals": True,
            },
            "spec_states": 6,
            "impl_states": 4,
            "violations": [],
        })
        client = ConcirClient(
            self.stub,
            workdir=self.root / "calls",
            env=stub_env(STUB_STDOUT=ref_stdout, STUB_EXIT="0"),
        )
        plugin = SpnMirRefinementPlugin(client=client, spn_binary=self.fake_spn)
        fake_run, captured = self._mock_cargo_exporting_net(
            {"schema": "concir-impl-net/1", "states": [], "edges": [], "truncated": False}
        )
        with mock.patch("concplan.code_verify.static_verify.subprocess.run", side_effect=fake_run):
            rep = plugin.verify(
                skeleton_dir=self.skel,
                filled_dir=self.filled,
                spec_path=self.spec,
                contract_path=self.contract,
                fills=self.fills,
            )
        self.assertEqual(rep.status, "pass")
        self.assertTrue(rep.checks["weak_simulation"])
        self.assertEqual(captured["argv"], ["cargo", "check", "--offline", "--no-default-features"])
        self.assertEqual(captured["env"]["RUSTC_WRAPPER"], str(self.fake_spn.resolve()))
        self.assertIn("--export-concir-net", captured["env"]["SPN_FLAGS"])
        self.assertIn("--codegen-map", captured["env"]["SPN_FLAGS"])

    def test_spn_plugin_fail_attributes_violation_to_hole_via_span(self):
        # h2 spans lines 5..8 in filled/src/main.rs; report a violation at line 6 with no hole_id.
        ref_stdout = json.dumps({
            "schema": "concir-refinement-report/1",
            "outcome": "fail",
            "checks": {"hole_purity": True, "weak_simulation": False},
            "violations": [{
                "obligation": "O2_refinement",
                "kind": "unexpected_sync_step",
                "message": "unexpected mutex_lock(main::m)",
                "rust_span": "src/main.rs:6:5",
                "witness_trace": [{"step": 0, "op": "mutex_lock", "resource": "main::m"}],
                "expected_spec_ops": ["mutex_unlock(main::m)"],
            }],
        })
        client = ConcirClient(
            self.stub,
            workdir=self.root / "calls",
            env=stub_env(STUB_STDOUT=ref_stdout, STUB_EXIT="1"),
        )
        plugin = SpnMirRefinementPlugin(client=client, spn_binary=self.fake_spn)
        fake_run, _ = self._mock_cargo_exporting_net({"schema": "concir-impl-net/1", "states": []})
        with mock.patch("concplan.code_verify.static_verify.subprocess.run", side_effect=fake_run):
            rep = plugin.verify(
                skeleton_dir=self.skel,
                filled_dir=self.filled,
                spec_path=self.spec,
                contract_path=self.contract,
            )
        self.assertEqual(rep.status, "fail_refinement")
        self.assertEqual(rep.offending_holes, ["h2"])
        self.assertEqual(rep.violations[0].sid, "s2")
        self.assertEqual(rep.violations[0].req_clauses, ["R2"])
        self.assertEqual(rep.violations[0].expected_spec_ops, ["mutex_unlock(main::m)"])

    def test_spn_plugin_unknown_on_truncated(self):
        ref_stdout = json.dumps({
            "schema": "concir-refinement-report/1",
            "outcome": "unknown",
            "checks": {"weak_simulation": False},
            "violations": [],
        })
        client = ConcirClient(
            self.stub,
            workdir=self.root / "calls",
            env=stub_env(STUB_STDOUT=ref_stdout, STUB_EXIT="3"),
        )
        plugin = SpnMirRefinementPlugin(client=client, spn_binary=self.fake_spn)
        fake_run, _ = self._mock_cargo_exporting_net({"schema": "concir-impl-net/1", "truncated": True})
        with mock.patch("concplan.code_verify.static_verify.subprocess.run", side_effect=fake_run):
            rep = plugin.verify(
                skeleton_dir=self.skel,
                filled_dir=self.filled,
                spec_path=self.spec,
            )
        self.assertEqual(rep.status, "unknown")

    def test_composite_never_turns_unknown_into_pass(self):
        composite = CompositeStaticVerifier([
            AstHolePurityPlugin(),
            SpnMirRefinementPlugin(spn_binary=None),
        ])
        with mock.patch("concplan.code_verify.static_verify.find_spn", return_value=None):
            rep = composite.verify(
                skeleton_dir=self.skel,
                filled_dir=self.filled,
                spec_path=self.spec,
                fills=self.fills,
            )
        self.assertEqual(rep.status, "unknown")
        self.assertTrue(rep.checks["hole_purity"])
        self.assertFalse(rep.checks["spn_available"])

    def test_lockbud_plugin_unknown_clean_and_detected(self):
        with mock.patch("concplan.code_verify.static_verify.find_lockbud", return_value=None):
            rep = LockbudPlugin().verify(skeleton_dir=self.skel, filled_dir=self.filled)
        self.assertEqual(rep.status, "unknown")

        fake_lb = self.root / "bin" / ("lockbud.exe" if os.name == "nt" else "lockbud")
        fake_lb.write_text("lb", encoding="utf-8")
        plugin = LockbudPlugin(lockbud_binary=fake_lb)

        clean_proc = subprocess.CompletedProcess(["cargo"], 0, stdout="ok", stderr="")
        with mock.patch("concplan.code_verify.static_verify.subprocess.run", return_value=clean_proc):
            rep_ok = plugin.verify(skeleton_dir=self.skel, filled_dir=self.filled)
        self.assertEqual(rep_ok.status, "pass")

        bug_proc = subprocess.CompletedProcess(
            ["cargo"], 0, stdout='{"bug_kind": "ConflictLock"}', stderr=""
        )
        with mock.patch("concplan.code_verify.static_verify.subprocess.run", return_value=bug_proc):
            rep_bug = plugin.verify(skeleton_dir=self.skel, filled_dir=self.filled)
        self.assertEqual(rep_bug.status, "fail_refinement")
        self.assertEqual(rep_bug.violations[0].kind, "lockbud_ConflictLock")


if __name__ == "__main__":
    unittest.main()
