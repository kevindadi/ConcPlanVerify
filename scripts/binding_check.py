#!/usr/bin/env python3
"""Binding-check CLI over the instrumenter's structural metadata.

The AST analysis lives in ConcIR (`concir-instrument` emits each resource's
construction ``site`` and, for spawns, the thread ``entry`` plus whether that
entry is unambiguous). This CLI consumes that metadata and a versioned binding
manifest, and emits machine-readable verdicts:

- ``verified``   — the binding is supported by structural evidence;
- ``unresolved`` — no structural evidence; a suggestion may be recorded;
- ``violated``   — the manifest claims a binding the structure contradicts.

A manifest claim is a *hypothesis*: it is checked, not trusted. String rules
(exact, module-prefix, handle substring) are suggestions only.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

import subprocess  # noqa: E402

from cir_workflow.generation import mapping_to_cir  # noqa: E402


def run_rust(resources_path, cir_path, manifest_path=None, binary=None) -> dict | None:
    """Call the ConcIR ``bind_check`` CLI and return its JSON, or None if absent."""

    binary = Path(binary) if binary else REPO.parent / "ConcIR/target/release/bind_check"
    if not Path(binary).is_file():
        return None
    argv = [str(binary), "--resources", str(resources_path), "--cir", str(cir_path)]
    if manifest_path:
        argv += ["--manifest", str(manifest_path)]
    proc = subprocess.run(argv, capture_output=True, text=True)
    if proc.returncode != 0:
        return None
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None


def check(resources: list[dict], cir: dict, manifest: list[dict] | None) -> dict:
    mapping, prov = mapping_to_cir(resources, cir)
    ambiguous = {a["rust"]: a for a in prov.get("ambiguous", [])}
    verified = {rust: {"cir": fqn, "rule": prov["rules"].get(rust)}
                for rust, fqn in mapping.items()}
    unresolved = {}
    violated = {}
    by_name = {r.get("name"): r for r in resources}
    for claim in manifest or []:
        rust, cir_fqn = claim.get("rust"), claim.get("cir")
        res = by_name.get(rust)
        if res is None:
            violated[rust] = {"claim": cir_fqn, "reason": "no such runtime resource"}
            continue
        actual = mapping.get(rust)
        if actual == cir_fqn:
            verified[rust] = {"cir": cir_fqn, "rule": "manifest+structure",
                              "site": res.get("site")}
        elif rust in ambiguous:
            unresolved[rust] = {"claim": cir_fqn, "reason": ambiguous[rust].get("reason")
                                or "no structural evidence",
                                "suggestion": ambiguous[rust].get("suggestion")}
        else:
            violated[rust] = {"claim": cir_fqn, "actual": actual,
                              "reason": "manifest disagrees with structure"}
    for rust, a in ambiguous.items():
        unresolved.setdefault(rust, {"claim": None, "reason": a.get("reason")
                                     or "no structural evidence",
                                     "suggestion": a.get("suggestion"),
                                     "candidates": a.get("candidates"),
                                     "site": a.get("site")})
    return {"verified": verified, "unresolved": unresolved, "violated": violated}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--resources", required=True, help="instrument resources.json")
    parser.add_argument("--cir", required=True)
    parser.add_argument("--manifest", default=None, help="binding manifest JSON (list)")
    args = parser.parse_args()
    resources = json.loads(Path(args.resources).read_text())["resources"]
    cir = json.loads(Path(args.cir).read_text())
    manifest = json.loads(Path(args.manifest).read_text()) if args.manifest else None
    result = check(resources, cir, manifest)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
