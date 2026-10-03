"""Task-level structural fidelity checks on generated modular CIR.

These checks are deliberately *task templates*: they verify that the candidate
keeps the entry, the two spawned tasks, the shared-lock identity/ownership, the
exact acquisition and release order, and (for the cross-module task) the
cross-module declarations that the frozen task requires. They are independent of
the Rust verdict, so a model that quietly "fixes" the defect is flagged.

They do not attempt to be a general CIR interpreter. A structure outside the
frozen template (no scope, spawn/call instead of scope, control flow in a task)
is reported as ``ok: None`` (unknown), never as faithful.
"""

from __future__ import annotations

from typing import Any

MUTEX_LOCK = "mutex_lock"
MUTEX_UNLOCK = "mutex_unlock"
CONTROL_KINDS = {"goto", "branch", "switch", "select"}


def _modules(program: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {m.get("name"): m for m in program.get("modules", []) or []}


def _functions(program: dict[str, Any]) -> dict[str, dict[str, Any]]:
    out = {}
    for module in program.get("modules", []) or []:
        for function in module.get("functions", []) or []:
            out[f"{module.get('name')}::{function.get('name')}"] = function
    return out


def _split_fqn(ref: str) -> tuple[str, str] | None:
    if "::" in ref:
        mod, fn = ref.split("::", 1)
        return mod, fn
    return None


def _norm_ref(module: str, ref: str) -> str:
    return ref if "::" in ref else f"{module}::{ref}"


def _resource_owners(program: dict[str, Any]) -> dict[str, set[str]]:
    owners: dict[str, set[str]] = {}
    for module in program.get("modules", []) or []:
        for res in module.get("resources", []) or []:
            name = res.get("name")
            if name:
                owners.setdefault(name, set()).add(module.get("name"))
    return owners


def _res_fqn(program: dict[str, Any], module: str, ref: str) -> str | None:
    """Fully-qualified resource name, or None when ownership is ambiguous."""
    if "::" in ref:
        return ref
    owners = _resource_owners(program).get(ref, set())
    if len(owners) == 1:
        return f"{next(iter(owners))}::{ref}"
    if module in owners:
        return f"{module}::{ref}"
    return None


def _entry(program: dict[str, Any]) -> tuple[str, str] | None:
    entry = program.get("entry")
    if isinstance(entry, str) and "::" in entry:
        mod, fn = entry.split("::", 1)
        return mod, fn
    return None


def _entry_scope(program: dict[str, Any]) -> tuple[str, list[str], str | None]:
    """Return (kind, spawned_fqns, reason): kind is ``scope``/``other``/``none``."""
    ent = _entry(program)
    if ent is None:
        return "none", [], "program has no valid `entry` FQN"
    mod, fn = ent
    functions = _functions(program)
    function = functions.get(f"{mod}::{fn}")
    if function is None:
        return "none", [], f"entry function {mod}::{fn} is not declared"
    scopes = [s for s in function.get("body", []) or [] if s.get("kind") == "scope"]
    starters = [s for s in function.get("body", []) or []
                if s.get("kind") in {"spawn", "call", "async_call"}]
    if len(scopes) == 1 and not starters:
        refs = [str(r) for r in scopes[0].get("funcs", []) or []]
        return "scope", [_norm_ref(mod, r) for r in refs], None
    if not scopes and starters:
        return "other", [], "concurrent tasks are started with spawn/call, not a scope"
    if not scopes:
        return "none", [], "entry function has no scope statement starting the tasks"
    return "other", [], "entry function has more than one scope statement"


def _lock_sequence(program: dict[str, Any], fqn: str) -> list[tuple[str, str]] | None:
    function = _functions(program).get(fqn)
    if function is None:
        return None
    if any(s.get("kind") in CONTROL_KINDS for s in function.get("body", []) or []):
        return None
    module = fqn.split("::")[0]
    seq = []
    for stmt in function.get("body", []) or []:
        kind = stmt.get("kind")
        if kind in (MUTEX_LOCK, MUTEX_UNLOCK):
            res = _res_fqn(program, module, str(stmt.get("resource")))
            if res is None:
                return None
            seq.append((kind, res))
    return seq


def _exact_order(seq: list[tuple[str, str]] | None, expected: list[str]) -> tuple[bool, str | None]:
    if seq is None:
        return False, "task function is missing, or contains control flow / ambiguous resources"
    if seq != [(MUTEX_LOCK, expected[0]), (MUTEX_LOCK, expected[1]),
               (MUTEX_UNLOCK, expected[1]), (MUTEX_UNLOCK, expected[0])]:
        return False, f"lock/unlock sequence is {seq}, expected exactly {expected} then reverse"
    return True, None


def _check_pair_task(program: dict[str, Any], *, t1_order: list[str], t2_order: list[str],
                     t1_fqn: str = "main::t1", t2_fqn: str = "main::t2",
                     require_cross_refs: bool = False) -> dict[str, Any]:
    criteria: dict[str, Any] = {}
    reasons: list[str] = []

    ent = _entry(program)
    criteria["entry_ok"] = ent == ("main", "main")
    if not criteria["entry_ok"]:
        reasons.append(f"entry is {program.get('entry')!r}, expected 'main::main'")

    kind, spawned, scope_reason = _entry_scope(program)
    criteria["scope_kind"] = kind
    if kind != "scope":
        reasons.append(scope_reason or "entry does not use a single scope statement")
        criteria["scope_ok"] = False
    else:
        criteria["scope_ok"] = set(spawned) == {t1_fqn, t2_fqn}
        if not criteria["scope_ok"]:
            reasons.append(f"scope spawns {spawned}, expected exactly [{t1_fqn}, {t2_fqn}]")

    seq1 = _lock_sequence(program, t1_fqn)
    seq2 = _lock_sequence(program, t2_fqn)
    ok1, why1 = _exact_order(seq1, t1_order)
    ok2, why2 = _exact_order(seq2, t2_order)
    criteria["t1_ok"] = ok1
    criteria["t2_ok"] = ok2
    if not ok1:
        reasons.append(f"{t1_fqn}: {why1}")
    if not ok2:
        reasons.append(f"{t2_fqn}: {why2}")

    if require_cross_refs:
        modmap = _modules(program)
        cross_ok = True
        for fqn, res in [(t1_fqn, t1_order[1]), (t2_fqn, t2_order[1])]:
            module = fqn.split("::")[0]
            req = [str(r) for r in (modmap.get(module, {}).get("requires", {}) or {})
                   .get("resources", []) or []]
            if res not in req:
                cross_ok = False
                reasons.append(f"{module} requires.resources does not declare {res}")
        criteria["cross_refs_ok"] = cross_ok

    known = all(criteria.get(k) is not None for k in
                ("entry_ok", "scope_ok", "t1_ok", "t2_ok"))
    if known and reasons == []:
        return {"ok": True, "reason": None, "criteria": criteria, "spawned": spawned}
    if kind == "other":
        return {"ok": None, "reason": "; ".join(reasons), "criteria": criteria, "spawned": spawned}
    return {"ok": False, "reason": "; ".join(reasons), "criteria": criteria, "spawned": spawned}


def check_t1_same_order(program: dict[str, Any]) -> dict[str, Any]:
    return _check_pair_task(program, t1_order=["main::a", "main::b"],
                            t2_order=["main::a", "main::b"])


def check_t2_abba(program: dict[str, Any]) -> dict[str, Any]:
    return _check_pair_task(program, t1_order=["main::a", "main::b"],
                            t2_order=["main::b", "main::a"])


def check_t3_cross_module_abba(program: dict[str, Any]) -> dict[str, Any]:
    result = _check_pair_task(
        program,
        t1_order=["main::a", "other::b"],
        t2_order=["other::b", "main::a"],
        t1_fqn="main::t1",
        t2_fqn="other::t2",
        require_cross_refs=True,
    )
    criteria = result["criteria"]
    modules = sorted(m.get("name") for m in program.get("modules", []) or [])
    criteria["modules"] = modules
    owners_ok = True
    owners = _resource_owners(program)
    if owners.get("a") != {"main"}:
        owners_ok = False
        result["reason"] = (result.get("reason") or "") + " ; module main must own resource 'a'"
    if owners.get("b") != {"other"}:
        owners_ok = False
        result["reason"] = (result.get("reason") or "") + " ; module other must own resource 'b'"
    if "main" not in modules or "other" not in modules:
        owners_ok = False
        result["reason"] = (result.get("reason") or "") + " ; expected modules main and other"
    criteria["ownership_ok"] = owners_ok and criteria.get("cross_refs_ok", False)
    if result["ok"] is True and not criteria["ownership_ok"]:
        result["ok"] = False
    return result


CHECKS = {
    "same_order_pair": check_t1_same_order,
    "abba_inversion": check_t2_abba,
    "cross_module_abba": check_t3_cross_module_abba,
}


def run_structural_check(name: str, program: dict[str, Any]) -> dict[str, Any]:
    check = CHECKS.get(name)
    if check is None:
        return {"ok": None, "reason": f"unknown structural check {name!r}", "criteria": {}}
    return check(program)
