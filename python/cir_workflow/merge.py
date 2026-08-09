"""Merge modular ConcIR fragments into one Program.

Modular generation splits a large program into independently generated
ConcIR fragments (one per source-file-sized module), then this module
assembles them into a single flat ``Program`` that the Rust translator can
verify. Only ConcIR is merged — the CVN stays a single net built from the
merged Program.

Each fragment is a regular ConcIR ``Program`` dict. Cross-module references
(`spawn`/`join`/`call` naming a function in another fragment) resolve
naturally once all fragments are combined; the merge enforces the global
invariants that make the result a valid input for ``cir::validate``:
function names unique, shared resources declared consistently, one entry
owner, goal ids unique.
"""

from __future__ import annotations

from typing import Any

# ── Resource consistency ─────────────────────────────────────────────────────


def _resource_signature(resource: dict[str, Any]) -> tuple[str, ...]:
    """Tuple used to compare same-named resource declarations across modules."""
    return (
        str(resource.get("kind")),
        str(resource.get("type")),
        str(resource.get("mode")),
        json_value_str(resource.get("count")),
        json_value_str(resource.get("base")),
        json_value_str(resource.get("init")),
    )


def json_value_str(value: Any) -> str:
    import json

    try:
        return json.dumps(value, sort_keys=True, ensure_ascii=False)
    except (TypeError, ValueError):
        return repr(value)


class MergeError(ValueError):
    """Raised when fragments cannot be combined into a valid Program."""


def merge_modules(
    modules: list[dict[str, Any]],
    *,
    program_name: str,
    entry_module: str,
) -> tuple[dict[str, Any], dict[str, str]]:
    """Assemble modular fragments into one ConcIR ``Program``.

    ``modules`` is a list of ``{"module": name, "concir": {Program dict}}``
    entries. Returns ``(merged_program, function_to_module)``. Every function
    in the merged program carries a ``module`` tag naming its source fragment.
    """
    module_names = [m["module"] for m in modules]
    if len(set(module_names)) != len(module_names):
        raise MergeError("module names must be unique")
    if entry_module not in module_names:
        raise MergeError(
            f"entry module '{entry_module}' is not among the modules "
            f"(have: {', '.join(module_names)})"
        )

    resources: dict[str, dict[str, Any]] = {}
    protection: dict[tuple[str, str], dict[str, Any]] = {}
    goals: dict[str, dict[str, Any]] = {}
    functions: list[dict[str, Any]] = []
    function_to_module: dict[str, str] = {}
    entry_fn: str | None = None

    for module in modules:
        mod_name = module["module"]
        fragment = module["concir"]

        # Functions must be globally unique; tag each with its source module.
        for fn in fragment.get("functions", []):
            fn_name = fn["name"]
            if fn_name in function_to_module:
                raise MergeError(
                    f"function '{fn_name}' is defined in both module "
                    f"'{function_to_module[fn_name]}' and module '{mod_name}'"
                )
            tagged = dict(fn)
            tagged["module"] = mod_name
            functions.append(tagged)
            function_to_module[fn_name] = mod_name

        # Shared resources: first declaration wins, later ones must match.
        for res in fragment.get("resources", []):
            name = res["name"]
            if name in resources:
                if _resource_signature(resources[name]) != _resource_signature(res):
                    raise MergeError(
                        f"resource '{name}' is declared inconsistently across modules: "
                        f"{resources[name]} vs {res}"
                    )
            else:
                resources[name] = res

        # Protection mapping: deduplicate identical (var, lock) pairs.
        for prot in fragment.get("protection", []):
            protection.setdefault((prot["var"], prot["lock"]), prot)

        # Goals: ids must be unique across modules.
        for goal in fragment.get("goals", []):
            goal_id = goal["id"]
            if goal_id in goals:
                raise MergeError(
                    f"goal id '{goal_id}' is declared in more than one module; "
                    "prefix goal ids per module"
                )
            goals[goal_id] = goal

        if mod_name == entry_module:
            fragment_entry = fragment.get("entry")
            if not fragment_entry:
                raise MergeError(
                    f"entry module '{mod_name}' does not declare an entry function"
                )
            entry_fn = fragment_entry

    if entry_fn is None:
        raise MergeError("no entry function resolved")

    merged: dict[str, Any] = {
        "program": program_name,
        "resources": list(resources.values()),
        "protection": list(protection.values()),
        "functions": functions,
        "entry": entry_fn,
    }
    if goals:
        merged["goals"] = list(goals.values())

    return merged, function_to_module


# ── Bundle / directory loading ────────────────────────────────────────────────


def load_module_bundle(bundle: dict[str, Any]) -> tuple[list[dict[str, Any]], str, str]:
    """Validate a module-bundle dict and return ``(modules, program_name, entry_module)``.

    Accepted shapes:

    .. code-block:: json

        {
          "program": "project",
          "entry_module": "main",
          "modules": [
            {"module": "main", "concir": { "...Program fragment..." }},
            {"module": "worker", "concir": { "...Program fragment..." }}
          ]
        }
    """
    if not isinstance(bundle, dict):
        raise MergeError("module bundle must be a JSON object")
    program_name = bundle.get("program")
    entry_module = bundle.get("entry_module")
    modules = bundle.get("modules")
    if not isinstance(program_name, str) or not program_name:
        raise MergeError("bundle requires a non-empty 'program' name")
    if not isinstance(entry_module, str) or not entry_module:
        raise MergeError("bundle requires a non-empty 'entry_module'")
    if not isinstance(modules, list) or not modules:
        raise MergeError("bundle requires a non-empty 'modules' array")
    for m in modules:
        if not isinstance(m, dict) or "module" not in m or "concir" not in m:
            raise MergeError("each module must be {'module': name, 'concir': {...}}")
        if not isinstance(m["concir"], dict):
            raise MergeError(f"module '{m.get('module')}' concir must be an object")
    return modules, program_name, entry_module
