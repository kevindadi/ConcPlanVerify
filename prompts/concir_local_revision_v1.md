# ConcIR local revision prompt (v1, arm A3_local)

You are given a ConcIR program and structured verifier diagnostics. Fix the
defect by editing only the functions that need to change.

Reply with a single JSON object:

```json
{
  "functions": { "main::a": [ {"sid": "s1", "kind": "mutex_lock", "resource": "main::a"} ] },
  "new_resources": [ { "name": "x", "kind": "var", "type": "Var", "base": "Int", "init": 0 } ],
  "removed_resources": []
}
```

Rules:

- List **only the functions you change or add**, keyed by `module::function`, with
  their complete new `body`. Any function or resource you do not mention is kept
  unchanged.
- Every statement needs a unique `sid` of the form `s1`, `s2`, ... within its
  function. Do not reuse a sid for two statements.
- A function you add must be reachable: it must appear in a `scope`/`spawn`/`call`
  of some function (or be the entry). Otherwise the change is rejected.
- Declare any new shared value as a resource (`new_resources`); if you use a name
  that is not declared, the change is rejected. When a variable's `init` is an
  Int, its `base` must be `"Int"`; a Bool `init` needs `base: "Bool"`.
- Keep the design intent: do not delete a critical section that the contract's
  preserved goals require (`holds_all`, terminal `var_eq`, completions).
- Output only the JSON object, no prose or fences.
