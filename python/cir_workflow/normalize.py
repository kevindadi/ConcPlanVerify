"""Deterministic Tier-1 normalisation of a candidate CIR program.

Only rewrites with a single unambiguous mapping are applied, and every rewrite is
recorded in ``normalizations`` so the feedback can tell the model exactly what was
changed. Sid names are **never** rewritten (that would break traceability); an
ill-formed sid is reported in ``sid_issues`` instead.
"""

from __future__ import annotations

import copy
import re
from typing import Any

# `s<n>` is the canonical sid; `L<n>` is an instrumenter label used by the
# tool-driven extraction protocol (both are left untouched by the normalizer).
SID_RE = re.compile(r"^(s|L)[0-9]+$")

# Field aliases (from the authoritative schema): (kind, wrong) -> right.
FIELD_ALIASES: dict[tuple[str, str], str] = {
    ("write_shared", "var"): "resource",
    ("read_shared", "var"): "resource",
    ("mutex_lock", "var"): "resource",
    ("mutex_unlock", "var"): "resource",
    ("semaphore_acquire", "sem"): "resource",
    ("semaphore_release", "sem"): "resource",
    ("condvar_wait", "mutex"): "lock",
    ("condvar_wait", "m"): "lock",
    ("condvar_notify", "condition"): "condvar",
    ("channel_send", "ch"): "channel",
    ("channel_recv", "ch"): "channel",
}

EXPR_KINDS = {"bool", "int", "float", "string", "str", "enum"}

# Resource `kind`/`mode` follow from `type` (authoritative schema):
# Var/Atomic are value resources, everything else is a synchronisation resource.
VALUE_TYPES = {"Var", "Atomic"}
SYNC_TYPES = {"Channel", "Condvar", "Mutex", "Semaphore"}
RESOURCE_SYNC_MODE = "Sync"

# Statement fields that must be scalar strings. A single-element list holding a
# string is an unambiguous model serialisation slip and is unwrapped; any other
# list is a genuine error and is reported (and, in strict mode, rejected).
SCALAR_FIELDS = ("expr", "cond", "value", "expected", "desired", "resource",
                 "channel", "lock", "condvar", "func", "var", "dst", "src",
                 "then", "else", "default", "target", "handle")
DROP_EMPTY_STRING_LIST = ("else", "default")
FUNC_FIELDS = {"spawn": ("func",), "call": ("func",), "scope": ("funcs",)}
DEFAULT_PARAM_TYPE = "Int"

# protection entry aliases: {wrong} -> {var|lock}
PROTECTION_ALIASES = {"resource": "var", "protected_by": "lock", "mutex": "lock",
                      "lock_var": "lock", "variable": "var"}
PRIMITIVE_BASE_CASE = {"bool": "Bool", "int": "Int", "float": "Float",
                       "string": "String"}
# Rust-ish type spellings the model may use -> canonical CIR primitive names.
TYPE_ALIASES = {
    "bool": "Bool", "boolean": "Bool",
    "i8": "Int", "i16": "Int", "i32": "Int", "i64": "Int", "i128": "Int",
    "u8": "Int", "u16": "Int", "u32": "Int", "u64": "Int", "u128": "Int",
    "usize": "Int", "isize": "Int", "int": "Int",
    "f32": "Float", "f64": "Float", "float": "Float",
    "str": "String", "string": "String", "String": "String", "&str": "String",
}


def _canonical_type(value: Any) -> tuple[Any, bool]:
    if not isinstance(value, str):
        return value, False
    if value in ("Int", "Bool", "Float", "String"):
        return value, False
    if value in TYPE_ALIASES:
        return TYPE_ALIASES[value], TYPE_ALIASES[value] != value
    return "Int", True
# fields that are meaningless (and rejected) on a var/atomic resource
DROP_ON_VALUE_RESOURCE = ("mode",)


def _expr_string(value: Any) -> tuple[Any, dict | None]:
    """Turn an ``{kind:..,value:..}`` expression object into a source string."""

    if not isinstance(value, dict):
        return value, None
    kind = str(value.get("kind", "")).lower()
    if kind not in EXPR_KINDS or "value" not in value:
        return value, None
    raw = value["value"]
    if kind == "bool":
        text = "true" if raw else "false"
    elif kind in ("int", "float"):
        text = str(raw)
    else:
        text = '"%s"' % str(raw).replace('"', '\\"')
    return text, {"field": None, "from": value, "to": text, "rule": "expr_object_to_string"}


def _infer_base(resource: dict) -> dict | None:
    if resource.get("kind") != "var" or "base" in resource:
        return None
    if "init" not in resource:
        return None
    init = resource["init"]
    if isinstance(init, bool):
        base = "Bool"
    elif isinstance(init, int):
        base = "Int"
    elif isinstance(init, str):
        base = "String"
    else:
        return None
    return {"rule": "infer_base", "resource": resource.get("name"), "base": base}


def _pointer(module_idx: int, function_idx: int | None = None,
             statement_idx: int | None = None, field: str | None = None) -> str:
    pointer = f"/modules/{module_idx}"
    if function_idx is not None:
        pointer += f"/functions/{function_idx}"
    if statement_idx is not None:
        pointer += f"/body/{statement_idx}"
    if field:
        pointer += f"/{field}"
    return pointer


def _as_list(value: Any, name_key: str = "name") -> list | None:
    """Coerce a mapping-keyed collection into a list of items.

    Models frequently serialise `modules`/`resources`/`functions` as an object
    keyed by name; each value keeps its own `name` field or inherits the key.
    """

    if isinstance(value, list):
        return value
    if not isinstance(value, dict):
        return None
    items = []
    for key, item in value.items():
        if isinstance(item, dict):
            item = dict(item)
            item.setdefault(name_key, key)
            items.append(item)
        else:
            items.append({name_key: key, "value": item})
    return items


def _strip_module_prefix(name: Any, module_name: str | None) -> Any:
    """`main::main` inside module `main` is really the function `main`."""

    if not isinstance(name, str) or not module_name:
        return name
    prefix = f"{module_name}::"
    while name.startswith(prefix):
        name = name[len(prefix):]
    return name


def _function_fqns(modules: list) -> dict[str, list[str]]:
    """Map bare function name -> [module::name, ...] across the program."""

    by_name: dict[str, list[str]] = {}
    for module in modules:
        if not isinstance(module, dict):
            continue
        mname = module.get("name")
        for function in module.get("functions", []) or []:
            if isinstance(function, dict) and function.get("name"):
                by_name.setdefault(str(function["name"]), []).append(
                    f"{mname}::{function['name']}")
    return by_name


def _fix_fqn(name: Any, module_name: str | None,
             fn_fqns: dict[str, list[str]]) -> Any:
    """Prefix a bare function name with its owning module (``module::name``)."""

    if not isinstance(name, str) or not name or "::" in name:
        return name
    candidates = fn_fqns.get(name) or []
    if len(candidates) == 1:
        return candidates[0]
    if module_name:
        return f"{module_name}::{name}"
    return name


def _normalize_params(params: Any, function: dict,
                      records: list[dict], location: str) -> None:
    """Rewrite ``params`` into ``[{name, type}, ...]`` where unambiguous."""

    if not isinstance(params, list):
        return
    changed = False
    fixed = []
    for i, param in enumerate(params):
        if isinstance(param, str):
            fixed.append({"name": param, "type": DEFAULT_PARAM_TYPE})
            records.append({"rule": "param_string", "location": f"{location}/{i}",
                            "from": param, "to": {"name": param,
                                                  "type": DEFAULT_PARAM_TYPE}})
            changed = True
        elif isinstance(param, dict) and param.get("name") and "type" not in param:
            new = dict(param)
            new["type"] = DEFAULT_PARAM_TYPE
            fixed.append(new)
            records.append({"rule": "param_missing_type", "location": f"{location}/{i}",
                            "to": DEFAULT_PARAM_TYPE})
            changed = True
        elif isinstance(param, dict) and "type" in param:
            new = dict(param)
            canon, was = _canonical_type(new["type"])
            if was:
                new["type"] = canon
                records.append({"rule": "param_type", "location": f"{location}/{i}",
                                "from": param["type"], "to": canon})
                changed = True
            fixed.append(new)
        else:
            fixed.append(param)
    if changed:
        function["params"] = fixed


def normalize(program: dict, *, strict_arrays: bool = False
              ) -> tuple[dict, list[dict], list[dict]]:
    """Return ``(normalized, normalizations, sid_issues)``.

    With ``strict_arrays=True`` a scalar field serialised as a multi-element or
    non-string array is recorded as a rejection issue (with a JSON pointer)
    instead of being silently left for the verifier.
    """

    out = copy.deepcopy(program)
    records: list[dict] = []
    sid_issues: list[dict] = []

    # Top-level fields the verifier rejects: drop and record.
    allowed = {"program", "version", "entry", "modules"}
    for key in [k for k in list(out.keys()) if k not in allowed]:
        out.pop(key, None)
        records.append({"rule": "drop_top_level", "field": key})

    # Version must be the supported schema version.
    if out.get("version") != "3.5.0":
        records.append({"rule": "version", "from": out.get("version"), "to": "3.5.0"})
        out["version"] = "3.5.0"

    # Coerce name-keyed maps into lists (models often serialise collections as
    # objects) and strip a redundant `module::` prefix from function names.
    if isinstance(out.get("modules"), dict):
        coerced = _as_list(out.get("modules"))
        if coerced is not None:
            out["modules"] = coerced
            records.append({"rule": "collection_map_to_list", "scope": "modules"})
    for mi, module in enumerate(out.get("modules", []) or []):
        if not isinstance(module, dict):
            continue
        for coll in ("resources", "functions", "protection"):
            if isinstance(module.get(coll), dict):
                coerced = _as_list(module[coll])
                if coerced is not None:
                    module[coll] = coerced
                    records.append({"rule": "collection_map_to_list", "scope": coll,
                                    "location": _pointer(mi)})
        for fi, function in enumerate(module.get("functions", []) or []):
            if not isinstance(function, dict):
                continue
            stripped = _strip_module_prefix(function.get("name"), module.get("name"))
            if stripped != function.get("name"):
                records.append({"rule": "strip_module_prefix",
                                "location": _pointer(mi, fi),
                                "from": function.get("name"), "to": stripped})
                function["name"] = stripped
            if isinstance(function.get("body"), dict):
                coerced = _as_list(function["body"], name_key="sid")
                if coerced is not None:
                    function["body"] = coerced
                    records.append({"rule": "collection_map_to_list", "scope": "body",
                                    "location": _pointer(mi, fi)})
    fn_fqns = _function_fqns(out.get("modules", []) or [])

    # Entry must be a `module::function` FQN.
    entry = out.get("entry")
    if isinstance(entry, str) and "::" not in entry:
        owner = None
        for m in out.get("modules", []) or []:
            if any(isinstance(f, dict) and f.get("name") == entry
                   for f in m.get("functions", []) or []):
                owner = m.get("name")
                break
        if owner is None and out.get("modules"):
            owner = out["modules"][0].get("name")
        if owner:
            out["entry"] = f"{owner}::{entry}"
            records.append({"rule": "entry_fqn", "from": entry, "to": out["entry"]})

    for module_idx, module in enumerate(out.get("modules", []) or []):
        if not isinstance(module, dict):
            continue
        for resource in module.get("resources", []) or []:
            if not isinstance(resource, dict):
                continue
            rtype = resource.get("type")
            if rtype in VALUE_TYPES:
                inferred_kind = "var"
            elif rtype in SYNC_TYPES:
                inferred_kind = "sync"
            else:
                inferred_kind = None
            if inferred_kind and "kind" not in resource:
                resource["kind"] = inferred_kind
                records.append({"rule": "resource_kind", "scope": "resource",
                                "resource": resource.get("name"),
                                "location": _pointer(module_idx), "to": inferred_kind})
            if resource.get("kind") == "sync" and "mode" not in resource:
                resource["mode"] = RESOURCE_SYNC_MODE
                records.append({"rule": "resource_mode", "scope": "resource",
                                "resource": resource.get("name"),
                                "location": _pointer(module_idx),
                                "to": RESOURCE_SYNC_MODE})
            if resource.get("kind") == "sync" and "init" in resource:
                resource.pop("init", None)
                records.append({"rule": "drop_sync_init", "resource": resource.get("name"),
                                "location": _pointer(module_idx)})
            if isinstance(resource.get("base"), str):
                canon, _ = _canonical_type(resource["base"])
                if canon in ("Int", "Bool", "Float", "String"):
                    resource["base"] = canon
            if resource.get("kind") == "var":
                for field in DROP_ON_VALUE_RESOURCE:
                    if field in resource:
                        resource.pop(field)
                        records.append({"rule": "drop_field", "scope": "resource",
                                        "resource": resource.get("name"), "field": field})
            base = resource.get("base")
            if isinstance(base, str) and base in PRIMITIVE_BASE_CASE:
                resource["base"] = PRIMITIVE_BASE_CASE[base]
                records.append({"rule": "base_case", "resource": resource.get("name"),
                                "to": resource["base"]})
            inferred = _infer_base(resource)
            if inferred:
                resource["base"] = inferred["base"]
                records.append(inferred)
        for entry in module.get("protection", []) or []:
            for wrong, right in PROTECTION_ALIASES.items():
                if wrong in entry and right not in entry:
                    entry[right] = entry.pop(wrong)
                    records.append({"rule": "protection_alias", "scope": "protection",
                                    "from": wrong, "to": right})
        for fn_idx, function in enumerate(module.get("functions", []) or []):
            if not isinstance(function, dict):
                continue
            if "kind" not in function:
                function["kind"] = "normal"
                records.append({"rule": "function_kind", "function": function.get("name"),
                                "location": _pointer(module_idx, fn_idx)})
            _normalize_params(function.get("params"), function, records,
                              _pointer(module_idx, fn_idx, None, "params"))
            for local in function.get("locals", []) or []:
                if isinstance(local, dict) and "type" in local:
                    canon, changed = _canonical_type(local["type"])
                    if changed:
                        records.append({"rule": "local_type", "function": function.get("name"),
                                        "from": local["type"], "to": canon})
                        local["type"] = canon
            slot_names: set[str] = set()
            body = function.get("body", []) or []
            # Deterministic sid repair: fill missing / rename malformed sids and
            # rewrite goto/branch/switch targets accordingly.
            sid_map: dict[str, str] = {}
            used = {str(s.get("sid")) for s in body if SID_RE.match(str(s.get("sid", "")))}
            n = 1
            for stmt in body:
                sid = stmt.get("sid")
                if sid is None or not SID_RE.match(str(sid)):
                    while f"s{n}" in used:
                        n += 1
                    new = f"s{n}"
                    used.add(new)
                    n += 1
                    if sid is not None:
                        sid_map[str(sid)] = new
                    stmt["sid"] = new
                    records.append({"rule": "sid_rename", "function": function.get("name"),
                                    "from": sid, "to": new})
            if sid_map:
                for stmt in body:
                    target = stmt.get("target")
                    if isinstance(target, str) and target in sid_map:
                        stmt["target"] = sid_map[target]
                    if stmt.get("kind") == "branch":
                        for key in ("then", "else"):
                            if isinstance(stmt.get(key), str) and stmt[key] in sid_map:
                                stmt[key] = sid_map[stmt[key]]
                    if stmt.get("kind") == "switch":
                        cases = stmt.get("cases") or {}
                        for k, v in list(cases.items()):
                            if isinstance(v, str) and v in sid_map:
                                cases[k] = sid_map[v]
                        if isinstance(stmt.get("default"), str) and stmt["default"] in sid_map:
                            stmt["default"] = sid_map[stmt["default"]]
            for stmt_idx, stmt in enumerate(body):
                if not isinstance(stmt, dict):
                    continue
                sid = str(stmt.get("sid", ""))
                kind = str(stmt.get("kind", ""))
                for wrong, right in list(FIELD_ALIASES.items()):
                    (alias_kind, bad) = wrong
                    if kind == alias_kind and bad in stmt and right not in stmt:
                        stmt[right] = stmt.pop(bad)
                        records.append({"rule": "field_alias", "function": function.get("name"),
                                        "sid": sid, "from": bad, "to": right})
                # FQN-prefix bare function references (spawn.func / call.func /
                # scope.funcs) using the program-wide name map, local first.
                for ffield in FUNC_FIELDS.get(kind, ()):  # noqa: B007
                    value = stmt.get(ffield)
                    if isinstance(value, list):
                        for i, item in enumerate(value):
                            fixed = _fix_fqn(item, module.get("name"), fn_fqns)
                            if fixed != item:
                                value[i] = fixed
                                records.append({"rule": "fqn_prefix",
                                                "function": function.get("name"),
                                                "sid": sid, "field": ffield, "from": item,
                                                "to": fixed})
                    else:
                        fixed = _fix_fqn(value, module.get("name"), fn_fqns)
                        if fixed != value:
                            stmt[ffield] = fixed
                            records.append({"rule": "fqn_prefix",
                                            "function": function.get("name"),
                                            "sid": sid, "field": ffield, "from": value,
                                            "to": fixed})
                dst_field = None
                if kind == "assign_local":
                    dst_field = "target"
                elif kind in ("read_shared", "atomic_load", "atomic_cas",
                              "channel_recv"):
                    dst_field = "dst"
                if dst_field and isinstance(stmt.get(dst_field), str):
                    slot_names.add(stmt[dst_field])
                for field in SCALAR_FIELDS:
                    if field not in stmt:
                        continue
                    value = stmt[field]
                    if isinstance(value, list):
                        location = _pointer(module_idx, fn_idx, stmt_idx, field)
                        if (len(value) == 1 and isinstance(value[0], str)):
                            stmt[field] = value[0]
                            records.append({"rule": "array_unwrap", "function": function.get("name"),
                                            "sid": sid, "field": field,
                                            "location": location, "to": value[0]})
                        elif not value and field in DROP_EMPTY_STRING_LIST:
                            stmt.pop(field, None)
                            records.append({"rule": "drop_empty_field",
                                            "function": function.get("name"), "sid": sid,
                                            "field": field, "location": location})
                        elif strict_arrays:
                            sid_issues.append({
                                "stage": "normalize", "pointer": location,
                                "reason": "expected a string, got an array",
                                "rule": "reject_array_field", "field": field})
                        continue
                    if field in ("expr", "cond", "value", "expected", "desired"):
                        new_value, record = _expr_string(value)
                        if record:
                            stmt[field] = new_value
                            record.update({"function": function.get("name"), "sid": sid,
                                           "field": field})
                            records.append(record)
            declared = {l.get("name") for l in (function.get("locals") or [])
                        if isinstance(l, dict)}
            new_locals = [{"name": name, "type": DEFAULT_PARAM_TYPE}
                          for name in sorted(slot_names) if name not in declared]
            if new_locals:
                function.setdefault("locals", [])
                function["locals"].extend(new_locals)
                records.append({"rule": "add_local", "function": function.get("name"),
                                "names": [l["name"] for l in new_locals]})
    return out, records, sid_issues


def render_normalizations(records: list[dict]) -> str:
    if not records:
        return "(none)"
    return "; ".join(json_dumps(r) for r in records)


def json_dumps(record: dict) -> str:
    import json

    return json.dumps(record, ensure_ascii=False, sort_keys=True)
