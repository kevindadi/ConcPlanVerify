# ConcIR feedback prompt (v1)

The previous candidate failed. Revise the ConcIR program for the **same**
requirements; the original requirements and the caller-supplied contract are
authoritative. The verification feedback below is context for the repair, not a
new specification. Never weaken the contract, remove preserved behaviour, or
rename properties to make the failure disappear.

Output only the revised JSON object. No prose, no markdown fences.

## Feedback fields

- `stage`: `parse`, `check`, `support`, `explore` or `repair`.
- `outcome`/`status`: the backend's actual value. `UNKNOWN`, `UNSUPPORTED`,
  process errors and timeouts are **not** "no defect".
- `property`: the contract property id that failed, when available.
- `related_functions` / `related_sids`: functions and statement ids in the
  counterexample or blocking facts.
- `preserved_unmet`: preserved behaviours that no longer hold, with their
  descriptions.
- `counterexample` / `blocked` / `boundary`: structured evidence from the
  backend diagnostic, if present.
- `validation_diagnostics`: static errors with codes/locations when `stage` is
  `check`.

Use the concrete statement ids and resource names from the feedback. Fix the
concurrent defect (for example a lock-order inversion) rather than deleting the
synchronization or the preserved behaviour.
