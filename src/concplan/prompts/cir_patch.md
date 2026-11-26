# ConcIR single-patch prompt (v1)

You propose **exactly one** constrained change to a frozen ConcIR program. A
separate Rust tool applies and fully verifies it against a frozen contract; you
never decide the verdict. Output only the JSON object, no prose or fences.

## What you may change

- One `swap_statements` between two **adjacent** `mutex_lock` statements in one
  function, where the two resources differ. This is the only allowed operation.
- Nothing else: no statement deletion, no whole-program replacement, no new or
  removed functions/resources/scopes, no contract change, no extra edits, no
  change to `spawn`/`scope`/`protection`.

## Output schema

```json
{
  "schema_version": "concir-external-patch-candidate-v1",
  "context_fingerprint": "<copy the context_fingerprint from the repair context>",
  "patch": {
    "module": "<module name from the context>",
    "function": "<function name from the context>",
    "original_hash": "<copy the function's original_hash from the context>",
    "changes": [{"kind": "swap_statements", "a": "<sid>", "b": "<sid>"}]
  }
}
```

- `a` and `b` are the `sid`s of the two adjacent `mutex_lock` statements to swap.
- `original_hash` must be the target function's `original_hash` from the context;
  a wrong value is rejected by the tool.
- `context_fingerprint` must match the context; a mismatch is rejected.

The tool re-checks the hash, the allowed scope, adjacency, resources, static
validity, support and the **full contract** (all properties and preserved
behaviour). If it rejects, you will receive the real structured reason and the
frozen context again; propose a different single patch. Do not fabricate a
verification result.
