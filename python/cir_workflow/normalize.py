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
            for stmt in function.get("body", []) or []:
                sid = str(stmt.get("sid", ""))
                if not SID_RE.match(sid):
                    sid_issues.append({"function": function.get("name"),
                                       "sid": stmt.get("sid")})
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
