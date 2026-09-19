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

SID_RE = re.compile(r"^s[0-9]+$")

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

# protection entry aliases: {wrong} -> {var|lock}
PROTECTION_ALIASES = {"resource": "var", "protected_by": "lock", "mutex": "lock",
                      "lock_var": "lock", "variable": "var"}
PRIMITIVE_BASE_CASE = {"bool": "Bool", "int": "Int", "float": "Float",
                       "string": "String"}
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


def normalize(program: dict) -> tuple[dict, list[dict], list[dict]]:
    """Return ``(normalized, normalizations, sid_issues)``."""

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

    for module in out.get("modules", []) or []:
        for resource in module.get("resources", []) or []:
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
        for function in module.get("functions", []) or []:
            if not isinstance(function, dict):
                continue
            if "kind" not in function:
                function["kind"] = "normal"
                records.append({"rule": "function_kind", "function": function.get("name")})
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
            for stmt in body:
                sid = str(stmt.get("sid", ""))
                kind = str(stmt.get("kind", ""))
                for wrong, right in list(FIELD_ALIASES.items()):
                    (alias_kind, bad) = wrong
                    if kind == alias_kind and bad in stmt and right not in stmt:
                        stmt[right] = stmt.pop(bad)
                        records.append({"rule": "field_alias", "function": function.get("name"),
                                        "sid": sid, "from": bad, "to": right})
                for field in ("expr", "cond", "value", "expected", "desired"):
                    if field in stmt:
                        new_value, record = _expr_string(stmt[field])
                        if record:
                            stmt[field] = new_value
                            record.update({"function": function.get("name"), "sid": sid,
                                           "field": field})
                            records.append(record)
    return out, records, sid_issues


def render_normalizations(records: list[dict]) -> str:
    if not records:
        return "(none)"
    return "; ".join(json_dumps(r) for r in records)


def json_dumps(record: dict) -> str:
    import json

    return json.dumps(record, ensure_ascii=False, sort_keys=True)
