"""Typed Python API wrapper for ``concir-backend`` subcommands."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from ..toolchain import resolve_binary_path
from .protocol import ConcirResult
from .runner import ConcirSubprocessRunner


class ConcirClient:
    """Invoke ``concir-backend`` from Python with a fresh directory per call."""

    def __init__(
        self,
        binary: Path | str,
        *,
        workdir: Path | str,
        timeout: float = 30.0,
        env: dict[str, str] | None = None,
        binary_sha256: str | None = None,
    ) -> None:
        self.binary = resolve_binary_path(binary)
        self.workdir = Path(workdir).expanduser().resolve()
        self.timeout = float(timeout)
        self.env = dict(os.environ if env is None else env)
        self._runner = ConcirSubprocessRunner(
            self.binary,
            workdir=self.workdir,
            timeout=self.timeout,
            env=self.env,
            binary_sha256=binary_sha256,
        )

    @property
    def binary_sha256(self) -> str:
        return self._runner.binary_sha256

    def _require_binary(self) -> None:
        self._runner.require_binary()

    def _call_dir(self, command: str) -> Path:
        return self._runner.call_dir(command)

    @staticmethod
    def _materialize(value: Path | str | dict[str, Any], dest: Path) -> Path:
        return ConcirSubprocessRunner.materialize(value, dest)

    @staticmethod
    def _prepare_out_path(out_path: Path | str | None) -> Path | None:
        return ConcirSubprocessRunner.prepare_out_path(out_path)

    def _invoke(
        self,
        command: str,
        argv: list[str],
        *,
        call_dir: Path,
        input_path: Path | None,
        contract_path: Path | None,
        config: dict[str, Any] | None = None,
        engine: str | None = None,
    ) -> ConcirResult:
        return self._runner.invoke(
            command,
            argv,
            call_dir=call_dir,
            input_path=input_path,
            contract_path=contract_path,
            config=config,
            engine=engine,
        )

    # ---- commands --------------------------------------------------------
    def check(self, program: Path | str) -> ConcirResult:
        call_dir = self._call_dir("check")
        program_path = self._materialize(program, call_dir / "program.json")
        return self._invoke(
            "check", ["check", str(program_path)],
            call_dir=call_dir, input_path=program_path, contract_path=None,
        )

    def support(self, program: Path | str) -> ConcirResult:
        call_dir = self._call_dir("support")
        program_path = self._materialize(program, call_dir / "program.json")
        return self._invoke(
            "support", ["support", str(program_path)],
            call_dir=call_dir, input_path=program_path, contract_path=None,
        )

    def explore(self, program: Path | str, contract: Path | str, engine: str = "petri") -> ConcirResult:
        if engine not in {"petri", "interp"}:
            raise ValueError(f"engine must be 'petri' or 'interp', got {engine!r}")
        call_dir = self._call_dir("explore")
        program_path = self._materialize(program, call_dir / "program.json")
        contract_path = self._materialize(contract, call_dir / "contract.json")
        return self._invoke(
            "explore",
            ["explore", str(program_path), str(contract_path), engine],
            call_dir=call_dir,
            input_path=program_path,
            contract_path=contract_path,
            engine=engine,
        )

    def repair(
        self,
        program: Path | str,
        contract: Path | str,
        *,
        strategy: str = "c",
        candidate_budget: int = 64,
        verification_budget: int = 64,
        max_depth: int = 4,
        max_total_edits: int = 4,
        artifact_name: str = "artifact.json",
    ) -> ConcirResult:
        if strategy not in {"a", "b", "c"}:
            raise ValueError(f"strategy must be 'a'/'b'/'c', got {strategy!r}")
        call_dir = self._call_dir("repair")
        program_path = self._materialize(program, call_dir / "program.json")
        contract_path = self._materialize(contract, call_dir / "contract.json")
        artifact_path = call_dir / artifact_name
        config = {
            "strategy": strategy,
            "candidate_budget": candidate_budget,
            "verification_budget": verification_budget,
            "max_depth": max_depth,
            "max_total_edits": max_total_edits,
        }
        result = self._invoke(
            "repair",
            [
                "repair", str(program_path), str(contract_path),
                "--strategy", strategy,
                "--candidate-budget", str(candidate_budget),
                "--verification-budget", str(verification_budget),
                "--max-depth", str(max_depth),
                "--max-total-edits", str(max_total_edits),
                "--artifact", str(artifact_path),
            ],
            call_dir=call_dir,
            input_path=program_path,
            contract_path=contract_path,
            config=config,
        )
        result.artifact_path = str(artifact_path) if artifact_path.exists() else None
        return result

    def repair_context(self, program: Path | str, contract: Path | str) -> ConcirResult:
        call_dir = self._call_dir("repair-context")
        program_path = self._materialize(program, call_dir / "program.json")
        contract_path = self._materialize(contract, call_dir / "contract.json")
        return self._invoke(
            "repair-context",
            [
                "repair-context", str(program_path), str(contract_path),
                "--artifact", str(call_dir / "context.json"),
            ],
            call_dir=call_dir,
            input_path=program_path,
            contract_path=contract_path,
        )

    def evaluate_patch(self, context: Path | str, candidate: Path | str) -> ConcirResult:
        call_dir = self._call_dir("evaluate-patch")
        context_path = self._materialize(context, call_dir / "context.json")
        candidate_path = self._materialize(candidate, call_dir / "candidate.json")
        result = self._invoke(
            "evaluate-patch",
            [
                "evaluate-patch", str(context_path), str(candidate_path),
                "--artifact", str(call_dir / "external_artifact.json"),
            ],
            call_dir=call_dir,
            input_path=candidate_path,
            contract_path=None,
        )
        artifact = call_dir / "external_artifact.json"
        result.artifact_path = str(artifact) if artifact.exists() else None
        return result

    def replay(self, artifact: Path | str) -> ConcirResult:
        call_dir = self._call_dir("replay")
        if isinstance(artifact, Path):
            artifact_path = Path(artifact).resolve()
        elif os.path.exists(str(artifact)) and "\n" not in str(artifact):
            artifact_path = Path(artifact).resolve()
        else:
            artifact_path = call_dir / "artifact.json"
            artifact_path.write_text(str(artifact), encoding="utf-8")
        return self._invoke(
            "replay", ["replay", str(artifact_path)],
            call_dir=call_dir, input_path=artifact_path, contract_path=None,
        )

    def lower(
        self,
        program_path: Path | str,
        *,
        out_path: Path | str | None = None,
        trace_map_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("lower")
        prog = self._materialize(program_path, call_dir / "program.json")
        argv = ["lower", str(prog)]
        resolved_out = self._prepare_out_path(out_path)
        resolved_tm = self._prepare_out_path(trace_map_path)
        if resolved_out is not None:
            argv += ["--out", str(resolved_out)]
        if resolved_tm is not None:
            argv += ["--trace-map", str(resolved_tm)]
        result = self._invoke(
            "lower", argv, call_dir=call_dir, input_path=prog, contract_path=None,
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        elif resolved_tm is not None and resolved_tm.exists():
            result.artifact_path = str(resolved_tm)
        return result

    def summarize(
        self,
        program_path: Path | str,
        *,
        module: str | None = None,
        out_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("summarize")
        prog = self._materialize(program_path, call_dir / "program.json")
        argv = ["summarize", str(prog)]
        config: dict[str, Any] = {}
        if module is not None:
            argv += ["--module", str(module)]
            config["module"] = str(module)
        resolved_out = self._prepare_out_path(out_path)
        if resolved_out is not None:
            argv += ["--out", str(resolved_out)]
        result = self._invoke(
            "summarize", argv, call_dir=call_dir, input_path=prog, contract_path=None, config=config,
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        return result

    def verify_refinement(
        self,
        spec_path: Path | str,
        impl_net_path: Path | str,
        *,
        contract_path: Path | str | None = None,
        codegen_map_path: Path | str | None = None,
        module: str | None = None,
        out_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("verify-refinement")
        spec = self._materialize(spec_path, call_dir / "spec.json")
        impl_net = self._materialize(impl_net_path, call_dir / "impl_net.json")
        argv = ["verify-refinement", "--spec", str(spec), "--impl-net", str(impl_net)]
        config: dict[str, Any] = {}
        contract_mat: Path | None = None
        if contract_path is not None:
            contract_mat = self._materialize(contract_path, call_dir / "contract.json")
            argv += ["--contract", str(contract_mat)]
        if codegen_map_path is not None:
            cmap_mat = self._materialize(codegen_map_path, call_dir / "codegen_map.json")
            argv += ["--codegen-map", str(cmap_mat)]
        if module is not None:
            argv += ["--module", str(module)]
            config["module"] = str(module)
        resolved_out = self._prepare_out_path(out_path)
        if resolved_out is not None:
            argv += ["--out", str(resolved_out)]
        result = self._invoke(
            "verify-refinement", argv, call_dir=call_dir,
            input_path=spec, contract_path=contract_mat, config=config,
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        return result

    def check_req(
        self,
        req_ir: Path | str | dict[str, Any],
        *,
        contract: Path | str | dict[str, Any] | None = None,
        out_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("check-req")
        req_mat = self._materialize(req_ir, call_dir / "req_ir.json")
        argv = ["check-req", str(req_mat)]
        contract_mat: Path | None = None
        if contract is not None:
            contract_mat = self._materialize(contract, call_dir / "contract.json")
            argv += ["--contract", str(contract_mat)]
        resolved_out = self._prepare_out_path(out_path)
        if resolved_out is not None:
            argv += ["--out", str(resolved_out)]
        result = self._invoke(
            "check-req", argv, call_dir=call_dir, input_path=req_mat, contract_path=contract_mat,
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        return result

    def verify_design(
        self,
        req_ir: Path | str | dict[str, Any],
        design: Path | str | dict[str, Any],
        *,
        contract: Path | str | dict[str, Any] | None = None,
        out_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("verify-design")
        req_mat = self._materialize(req_ir, call_dir / "req_ir.json")
        design_mat = self._materialize(design, call_dir / "design.json")
        argv = ["verify-design", "--req", str(req_mat), "--design", str(design_mat)]
        contract_mat: Path | None = None
        if contract is not None:
            contract_mat = self._materialize(contract, call_dir / "contract.json")
            argv += ["--contract", str(contract_mat)]
        resolved_out = self._prepare_out_path(out_path)
        if resolved_out is not None:
            argv += ["--out", str(resolved_out)]
        result = self._invoke(
            "verify-design", argv, call_dir=call_dir, input_path=design_mat, contract_path=contract_mat,
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        return result

    def explain_contract(
        self,
        contract: Path | str | dict[str, Any],
        *,
        out_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("explain-contract")
        contract_mat = self._materialize(contract, call_dir / "contract.json")
        argv = ["explain-contract", str(contract_mat)]
        resolved_out = self._prepare_out_path(out_path)
        if resolved_out is not None:
            argv += ["--output", str(resolved_out)]
        result = self._invoke(
            "explain-contract", argv, call_dir=call_dir, input_path=None, contract_path=contract_mat,
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        return result

    def validate_contract(
        self,
        contract: Path | str | dict[str, Any],
        *,
        requirements: Path | str | dict[str, Any] | None = None,
        strict_entities: bool = True,
        require_non_vacuous: bool = True,
        normalize: bool = False,
        out_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("validate-contract")
        contract_mat = self._materialize(contract, call_dir / "contract.json")
        argv = ["validate-contract", str(contract_mat)]
        if requirements is not None:
            req_mat = self._materialize(requirements, call_dir / "requirements.json")
            argv += ["--requirements", str(req_mat)]
        if not strict_entities:
            argv.append("--no-strict-entities")
        if not require_non_vacuous:
            argv.append("--no-require-non-vacuous")
        if normalize:
            argv.append("--normalize")
        resolved_out = self._prepare_out_path(out_path)
        if resolved_out is not None:
            argv += ["--output", str(resolved_out)]
        result = self._invoke(
            "validate-contract",
            argv,
            call_dir=call_dir,
            input_path=None,
            contract_path=contract_mat,
            config={
                "strict_entities": strict_entities,
                "require_non_vacuous": require_non_vacuous,
                "normalize": normalize,
            },
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        return result

    def export_cvn(
        self,
        program_path: Path | str,
        *,
        out_path: Path | str | None = None,
    ) -> ConcirResult:
        call_dir = self._call_dir("export-cvn")
        prog = self._materialize(program_path, call_dir / "program.json")
        argv = ["export-cvn", str(prog)]
        resolved_out = self._prepare_out_path(out_path)
        if resolved_out is not None:
            argv += ["--output", str(resolved_out)]
        result = self._invoke(
            "export-cvn", argv, call_dir=call_dir, input_path=prog, contract_path=None,
        )
        if resolved_out is not None and resolved_out.exists():
            result.artifact_path = str(resolved_out)
        return result
