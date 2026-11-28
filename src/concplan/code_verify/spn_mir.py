"""SPN MIR Petri-net extraction and stuttering refinement verifier plugin."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any

from ..core.concir_client import ConcirClient
from ..core.toolchain import find_concir_backend, find_spn
from .holes import (
    RUSTC_ERROR_SPAN_RE,
    attribute_span_to_hole,
    augment_codegen_map_with_hole_ranges,
    hole_meta_index,
    type_hole_meta_index,
)
from .verifier import StaticVerificationReport, StaticViolation, aggregate_statuses


def _format_spn_path(path: Path) -> str:
    """Format a path safely for ``SPN_FLAGS`` (parsed with Rust ``shellwords``)."""
    posix = path.as_posix()
    if any(ch.isspace() for ch in posix):
        return f'"{posix}"'
    return posix


def _detect_spn_toolchain(spn_path: Path, explicit: str | None = None) -> str | None:
    if explicit:
        return explicit
    env_tc = os.environ.get("CONCIR_SPN_TOOLCHAIN")
    if env_tc:
        return env_tc
    candidates = [
        spn_path.parent.parent.parent / "rust-toolchain.toml",
        Path(__file__).resolve().parents[3] / "backends/spn/rust-toolchain.toml",
    ]
    for tc_file in candidates:
        if tc_file.is_file():
            m = re.search(r'channel\s*=\s*"([^"]+)"', tc_file.read_text(encoding="utf-8"))
            if m:
                return m.group(1)
    return None


def _inject_toolchain_runtime_env(env: dict[str, str], toolchain: str | None) -> None:
    if not toolchain:
        return
    env.setdefault("RUSTUP_TOOLCHAIN", toolchain)
    rustup_home = Path(env.get("RUSTUP_HOME") or (Path.home() / ".rustup")) / "toolchains"
    if not rustup_home.is_dir():
        return
    matches = sorted(
        d for d in rustup_home.iterdir()
        if d.is_dir() and (d.name == toolchain or d.name.startswith(f"{toolchain}-"))
    )
    if not matches:
        return
    tc_dir = matches[0]
    bin_dir = tc_dir / "bin"
    lib_dir = tc_dir / "lib"
    path_sep = os.pathsep
    if bin_dir.is_dir():
        env["PATH"] = f"{bin_dir}{path_sep}{env.get('PATH', '')}"
    if os.name != "nt" and lib_dir.is_dir():
        ld_key = "DYLD_LIBRARY_PATH" if os.uname().sysname == "Darwin" else "LD_LIBRARY_PATH"
        env[ld_key] = f"{lib_dir}{path_sep}{env.get(ld_key, '')}"


class SpnMirRefinementPlugin:
    """Extract MIR Petri net with ``spn`` and verify stuttering refinement with ``concir-backend``."""

    name = "spn_mir_refinement"

    def __init__(
        self,
        *,
        client: ConcirClient | None = None,
        concir_binary: Path | str | None = None,
        spn_binary: Path | str | None = None,
        timeout: float = 180.0,
        cargo_offline: bool = True,
        spn_toolchain: str | None = None,
        modular: bool = False,
        focus_modules: list[str] | None = None,
    ) -> None:
        self.client = client
        self.concir_binary = concir_binary
        self.spn_binary = spn_binary
        self.timeout = float(timeout)
        self.cargo_offline = cargo_offline
        self.spn_toolchain = spn_toolchain
        self.modular = bool(modular)
        self.focus_modules = list(focus_modules) if focus_modules else None

    def _run_spn_and_refinement(
        self,
        *,
        filled: Path,
        spec: Path,
        contract_path: Path | None,
        cmap_path: Path,
        spn_path: Path,
        client: ConcirClient,
        hole_ranges: dict[str, dict[str, Any]],
        hole_meta: dict[str, dict[str, Any]],
        type_hole_ranges: dict[str, dict[str, Any]],
        type_hole_meta: dict[str, dict[str, Any]],
        work_dir: Path,
        focus_module: str | None = None,
    ) -> StaticVerificationReport:
        work_dir.mkdir(parents=True, exist_ok=True)
        impl_net_path = work_dir / "impl_net.json"
        if impl_net_path.exists():
            impl_net_path.unlink()
        refinement_out_path = work_dir / "refinement_report.json"

        target_dir = work_dir / "target-spn"
        if target_dir.exists():
            shutil.rmtree(target_dir, ignore_errors=True)

        argv = ["cargo"]
        if self.spn_toolchain:
            argv.append(f"+{self.spn_toolchain}")
        argv.append("check")
        if self.cargo_offline:
            argv.append("--offline")
        argv.append("--no-default-features")

        spn_flags = (
            f"--export-concir-net {_format_spn_path(impl_net_path)} "
            f"--codegen-map {_format_spn_path(cmap_path)}"
        )
        if focus_module:
            spn_flags += f" --focus-module {focus_module}"

        env = dict(os.environ)
        env["RUSTC_WRAPPER"] = str(spn_path)
        env["SPN_FLAGS"] = spn_flags
        env["CARGO_TARGET_DIR"] = str(target_dir)
        if self.cargo_offline:
            env["CARGO_NET_OFFLINE"] = "true"
        _inject_toolchain_runtime_env(env, _detect_spn_toolchain(spn_path, self.spn_toolchain))

        try:
            proc = subprocess.run(
                argv,
                cwd=str(filled),
                env=env,
                capture_output=True,
                text=True,
                timeout=self.timeout,
            )
        except subprocess.TimeoutExpired:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_completed": False},
                error=f"cargo/spn timed out after {self.timeout:g}s",
                metadata={"argv": argv, "spn_flags": spn_flags, "focus_module": focus_module},
            )
        except OSError as exc:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_completed": False},
                error=f"failed to invoke cargo/spn: {exc}",
                metadata={"argv": argv, "spn_flags": spn_flags, "focus_module": focus_module},
            )

        if not impl_net_path.is_file():
            combined_output = ((proc.stderr or "") + "\n" + (proc.stdout or "")).strip()
            stderr_tail = (proc.stderr or proc.stdout or "").strip()[-1500:]
            compile_violations: list[StaticViolation] = []
            if proc.returncode != 0:
                seen_spans: set[tuple[str, int, int, str]] = set()
                for m in RUSTC_ERROR_SPAN_RE.finditer(combined_output):
                    rel_file = m.group(1).replace("\\", "/")
                    line_no = int(m.group(2))
                    col_no = int(m.group(3))
                    rust_span = f"{rel_file}:{line_no}:{col_no}"
                    hid, sid, reqs = attribute_span_to_hole(
                        rust_span,
                        hole_ranges,
                        hole_meta,
                        type_hole_ranges=type_hole_ranges,
                        type_hole_meta=type_hole_meta,
                    )
                    if hid is None:
                        continue
                    key = (rel_file, line_no, col_no, hid)
                    if key in seen_spans:
                        continue
                    seen_spans.add(key)
                    compile_violations.append(
                        StaticViolation(
                            plugin=self.name,
                            obligation="O1_purity",
                            kind="compile_error_in_hole",
                            message=f"rustc compile error inside hole '{hid}' at {rust_span}: {stderr_tail}",
                            hole_id=hid,
                            sid=sid,
                            req_clauses=reqs,
                            rust_span=rust_span,
                        )
                    )

            if compile_violations:
                return StaticVerificationReport(
                    status="fail_hole",
                    plugin=self.name,
                    checks={"spn_available": True, "spn_exported": False},
                    violations=compile_violations,
                    error=f"cargo/spn compile error in hole(s) (exit {proc.returncode})",
                    metadata={
                        "argv": argv,
                        "spn_flags": spn_flags,
                        "focus_module": focus_module,
                        "cargo_exit_code": proc.returncode,
                        "cargo_stderr": stderr_tail,
                    },
                )

            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_exported": False},
                violations=[
                    StaticViolation(
                        plugin=self.name,
                        obligation="O2_refinement",
                        kind="build_failed" if proc.returncode != 0 else "missing_impl_net",
                        message=f"spn did not produce impl_net.json (cargo exit {proc.returncode}): {stderr_tail}",
                    )
                ],
                error=f"spn did not export {impl_net_path.name} (exit {proc.returncode})",
                metadata={
                    "argv": argv,
                    "spn_flags": spn_flags,
                    "focus_module": focus_module,
                    "cargo_exit_code": proc.returncode,
                    "cargo_stderr": stderr_tail,
                },
            )

        ref_kwargs: dict[str, Any] = {
            "contract_path": contract_path,
            "codegen_map_path": cmap_path,
            "out_path": refinement_out_path,
        }
        if focus_module is not None:
            ref_kwargs["module"] = focus_module
        ref_res = client.verify_refinement(spec, impl_net_path, **ref_kwargs)

        if ref_res.kind != "semantic":
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_exported": True},
                error=ref_res.error or f"verify-refinement {ref_res.kind}: {ref_res.status}",
                metadata={
                    "impl_net_path": str(impl_net_path),
                    "codegen_filled_path": str(cmap_path),
                    "focus_module": focus_module,
                },
            )

        payload = ref_res.payload or {}
        checks = {k: bool(v) for k, v in (payload.get("checks") or {}).items()}
        checks["spn_available"] = True
        checks["spn_exported"] = True

        metadata: dict[str, Any] = {
            "impl_net_path": str(impl_net_path),
            "codegen_filled_path": str(cmap_path),
            "refinement_report_path": (
                str(refinement_out_path) if refinement_out_path.is_file() else ref_res.stdout_path
            ),
            "spec_states": payload.get("spec_states"),
            "impl_states": payload.get("impl_states"),
            "relation": payload.get("relation"),
            "outcome": payload.get("outcome"),
        }
        if focus_module is not None:
            metadata["focus_module"] = focus_module

        if ref_res.status == "pass":
            return StaticVerificationReport(
                status="pass",
                plugin=self.name,
                checks=checks,
                metadata=metadata,
            )

        if ref_res.status == "unknown":
            return StaticVerificationReport(
                status="unknown",
                plugin=self.name,
                checks=checks,
                metadata=metadata,
                error=payload.get("message") or "refinement analysis returned unknown (e.g. truncated)",
            )

        violations: list[StaticViolation] = []
        for raw in payload.get("violations", []) or []:
            if not isinstance(raw, dict):
                continue
            rust_span = raw.get("rust_span")
            hole_id = raw.get("hole_id")
            sid = raw.get("sid")
            req_clauses = [str(r) for r in (raw.get("req_clauses") or [])]

            if not hole_id and rust_span:
                attr_hid, attr_sid, attr_reqs = attribute_span_to_hole(
                    str(rust_span),
                    hole_ranges,
                    hole_meta,
                    type_hole_ranges=type_hole_ranges,
                    type_hole_meta=type_hole_meta,
                )
                if attr_hid:
                    hole_id = attr_hid
                if not sid and attr_sid:
                    sid = attr_sid
                if not req_clauses and attr_reqs:
                    req_clauses = attr_reqs
            elif hole_id and (not sid or not req_clauses):
                meta = hole_meta.get(str(hole_id)) or type_hole_meta.get(str(hole_id)) or {}
                if not sid and meta.get("sid"):
                    sid = str(meta["sid"])
                if not req_clauses and meta.get("req"):
                    req_clauses = [str(r) for r in meta["req"]]

            violations.append(
                StaticViolation(
                    plugin=self.name,
                    obligation=str(raw.get("obligation") or "O2_refinement"),
                    kind=str(raw.get("kind") or "refinement_violation"),
                    message=str(raw.get("message") or "refinement check failed"),
                    hole_id=str(hole_id) if hole_id else None,
                    sid=str(sid) if sid else None,
                    req_clauses=req_clauses,
                    rust_span=str(rust_span) if rust_span else None,
                    witness_trace=list(raw.get("witness_trace") or []),
                    expected_spec_ops=[str(op) for op in (raw.get("expected_spec_ops") or [])],
                )
            )

        if not violations:
            violations.append(
                StaticViolation(
                    plugin=self.name,
                    obligation="O2_refinement",
                    kind=f"refinement_{ref_res.status}",
                    message=str(payload.get("message") or f"verify-refinement returned {ref_res.status}"),
                )
            )

        all_hole_purity = all(
            v.obligation == "O1_purity" or v.kind == "sync_in_hole"
            for v in violations
        )
        return StaticVerificationReport(
            status="fail_hole" if all_hole_purity else "fail_refinement",
            plugin=self.name,
            violations=violations,
            checks=checks,
            metadata=metadata,
        )

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        skeleton = Path(skeleton_dir).expanduser().resolve()
        filled = Path(filled_dir).expanduser().resolve()

        spn_path = find_spn(self.spn_binary)
        if spn_path is None:
            return StaticVerificationReport(
                status="unknown",
                plugin=self.name,
                checks={"spn_available": False},
                error="spn binary not found (set CONCIR_SPN or build backends/spn)",
            )

        if spec_path is None or not Path(spec_path).is_file():
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True},
                error="SpnMirRefinementPlugin requires an existing spec_path",
            )
        spec = Path(spec_path).expanduser().resolve()

        if codegen_map_path is not None and Path(codegen_map_path).is_file():
            cmap_path = Path(codegen_map_path).expanduser().resolve()
            cmap_data = json.loads(cmap_path.read_text(encoding="utf-8"))
        elif (filled / "codegen_filled.json").is_file():
            cmap_path = filled / "codegen_filled.json"
            cmap_data = json.loads(cmap_path.read_text(encoding="utf-8"))
        else:
            cmap_path = filled / "codegen_filled.json"
            if (filled / "src").is_dir():
                cmap_data = augment_codegen_map_with_hole_ranges(
                    skeleton, filled, fills, out_path=cmap_path
                )
            else:
                cmap_data = {"holes": [], "resources": [], "hole_ranges": {}}
                cmap_path.write_text(json.dumps(cmap_data, indent=2) + "\n", encoding="utf-8")

        hole_ranges: dict[str, dict[str, Any]] = cmap_data.get("hole_ranges", {}) or {}
        type_hole_ranges: dict[str, dict[str, Any]] = cmap_data.get("type_hole_ranges", {}) or {}
        hole_meta = hole_meta_index(cmap_data)
        type_hole_meta = type_hole_meta_index(cmap_data)

        spn_work = filled / ".spn-verify"
        spn_work.mkdir(parents=True, exist_ok=True)

        client = self.client
        if client is None:
            backend = find_concir_backend(self.concir_binary)
            if backend is None:
                return StaticVerificationReport(
                    status="unknown",
                    plugin=self.name,
                    checks={"spn_available": True},
                    error="concir-backend not found for verify-refinement",
                )
            client = ConcirClient(backend, workdir=spn_work / "concir-calls", timeout=self.timeout)

        target_modules: list[str] = []
        if self.focus_modules:
            target_modules = list(self.focus_modules)
        elif self.modular:
            try:
                spec_json = json.loads(spec.read_text(encoding="utf-8"))
                mod_names = [
                    str(m["name"])
                    for m in (spec_json.get("modules") or [])
                    if isinstance(m, dict) and m.get("name")
                ]
                if len(mod_names) > 1:
                    target_modules = mod_names
            except (OSError, json.JSONDecodeError):
                target_modules = []

        if not target_modules:
            return self._run_spn_and_refinement(
                filled=filled,
                spec=spec,
                contract_path=contract_path,
                cmap_path=cmap_path,
                spn_path=spn_path,
                client=client,
                hole_ranges=hole_ranges,
                hole_meta=hole_meta,
                type_hole_ranges=type_hole_ranges,
                type_hole_meta=type_hole_meta,
                work_dir=spn_work,
                focus_module=None,
            )

        all_violations: list[StaticViolation] = []
        merged_checks: dict[str, bool] = {"spn_available": True, "modular_refinement": True}
        statuses: list[str] = []
        errors: list[str] = []
        modular_reports: dict[str, dict[str, Any]] = {}
        global_summary_payload: dict[str, Any] | None = None

        sum_res = client.summarize(spec, out_path=spn_work / "global_summary.json")
        if sum_res.kind == "semantic":
            global_summary_payload = sum_res.payload
            merged_checks["global_lock_order_acyclic"] = sum_res.status == "pass"
            if sum_res.status == "fail":
                statuses.append("fail_refinement")
                all_violations.append(
                    StaticViolation(
                        plugin=self.name,
                        obligation="O2_refinement",
                        kind="cross_module_lock_cycle",
                        message="global module summary detected a cross-module lock-order cycle",
                    )
                )

        for mod_name in target_modules:
            rep = self._run_spn_and_refinement(
                filled=filled,
                spec=spec,
                contract_path=contract_path,
                cmap_path=cmap_path,
                spn_path=spn_path,
                client=client,
                hole_ranges=hole_ranges,
                hole_meta=hole_meta,
                type_hole_ranges=type_hole_ranges,
                type_hole_meta=type_hole_meta,
                work_dir=spn_work / f"mod-{mod_name}",
                focus_module=mod_name,
            )
            modular_reports[mod_name] = rep.as_dict()
            statuses.append(rep.status)
            all_violations.extend(rep.violations)
            for k, v in rep.checks.items():
                merged_checks[k] = merged_checks.get(k, True) and bool(v)
            if rep.error:
                errors.append(f"{mod_name}: {rep.error}")

        return StaticVerificationReport(
            status=aggregate_statuses(statuses),
            plugin=self.name,
            violations=all_violations,
            checks=merged_checks,
            metadata={
                "codegen_filled_path": str(cmap_path),
                "modular": True,
                "focus_modules": target_modules,
                "modular_reports": modular_reports,
                "global_summary": global_summary_payload,
            },
            error="; ".join(errors) if errors else None,
        )
