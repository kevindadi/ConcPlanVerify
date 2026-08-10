# cir2cvn JSON output contract

The `cir2cvn` binary (`--validate`, `--analyze`, `--goals`) prints a single JSON
object on stdout. This document defines its shape so an LLM can reliably act on
it.

## `--validate`

```jsonc
{
  "status": "valid" | "invalid_model",
  "valid": true | false,
  "diagnostics": [
    { "code": "E503", "severity": "error", "message": "...",
      "path": "functions[1].body[2].op", "fix_hint": "..." }
  ]
}
```

- Exit code `0` when valid, `1` when invalid, `2` on input/usage errors.
- Malformed JSON yields `status: "invalid_json"` with a single `E000`
  diagnostic.

## `--analyze` / `--goals`

```jsonc
{
  "status": "verified_safe" | "verified_unsafe" | "goals_unmet"
          | "invalid_model" | "translation_failed" | "analysis_incomplete",
  "validation": { /* concir ValidationReport */ },
  "translation_errors": ["T001: ..."],
  "translation_warnings": ["orphan control place: ..."],
  "places": 12,
  "transitions": 9,
  "places_by_kind": { "control": 8, "resource": 3, "wait": 1 },
  "input_arcs": 14,
  "output_arcs": 14,
  "cvn_dot": "digraph PetriNet { ... }",
  "state_count": 37,
  "analysis_complete": true,
  "max_states": 100000,
  "analysis_error": null,
  "bugs": [
    {
      "kind": {
        "Deadlock": { "participants": [
          { "function": "w1", "module": null, "blocked_at_sid": "w1.s2",
            "holding": ["mtx_a"], "waiting_for": "mtx_b" }
        ]}
      },
      "summary": "Deadlock detected involving w1, w2",
      "trace": [ { "transition_id": "...", "kind": "...", "anchor_sids": ["s1"],
                   "source_function": "w1", "module": null, "description": "lock (w1_s1_lock)" } ],
      "final_marking_summary": "{w1.s2, R(mtx_a)}",
      "involved_resources": ["mtx_a", "mtx_b"],
      "involved_functions": ["w1", "w2"],
      "involved_modules": [],
      "cir_slice": [ { "sid": "s2", "op": "ResOp { ... }", "function": "w1", "module": null } ],
      "preservation_constraints": ["Resource 'mtx_a' ..."],
      "repair_hint": "Enforce uniform lock ordering: ..."
    }
  ],
  "unmet_goals": [ { "goal": { "id": "g1", "desc": "...", "predicates": [...] },
                     "reason": "no reachable state satisfies all declared predicates" } ],
  "goal_warnings": ["goal 'g1' is already satisfied by the initial state ... (too weak)"],
  "declared_goal_count": 1,
  "timings": { "validation_ms": 0.1, "translation_ms": 0.2, "analysis_ms": 1.0,
               "goals_ms": 0.1, "total_ms": 1.5 }
}
```

## `bugs[].kind` variants

| Variant | JSON shape | Meaning |
| ------- | ---------- | ------- |
| `Deadlock` | `{"Deadlock": {"participants": [{function, module, blocked_at_sid, holding[], waiting_for}]}}` | No enabled transitions; ≥1 thread not at its thread-end place |
| `SignalLoss` | `{"SignalLoss": {"notifier_tid", "waiter_tid"}}` | A condvar notify fired before the waiter entered `wait` |
| `ChannelBlock` | `{"ChannelBlock": {"blocked_op": "send"\|"recv", "channel"}}` | send/recv has no matching counterpart |
| `DeadTransition` | `{"DeadTransition": {"transition", "sids"}}` | The anchored statement never fires on any interleaving |

## Error-code families

| Range | Meaning |
| ----- | ------- |
| `E000` | JSON parse / schema error |
| `E1xx–E7xx` | ConcIR static validation (names, types, resources, pairing, lock safety, control flow, protection) — each carries `fix_hint` |
| `T001–T302` | ConcIR → CVN translation errors (missing entry, unknown function, unknown resource type, invalid branch/switch, ambiguous RwLock drop, condvar no-wait-sites) |
| `V3xx` | Analysis-phase errors (token underflow, state-space explosion) |
| `V301` | insufficient tokens (fire) |
| `V302` | state space explosion: exceeded `max_states` |

## `analysis_incomplete`

Produced when exploration hit `max_states` (`analysis_error` = V302 message) or
an internal failure. `state_count` still reports how many states were explored.
Remedy: bound Int variables (`"base": {"Int": [lo, hi]}` on Var resources) or
raise `max_states`.
