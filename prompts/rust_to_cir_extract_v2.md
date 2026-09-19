# Rust -> CIR extraction prompt (v2)

You are given a Rust concurrent program. Produce:

1. a **ConcIR** model that follows the machine schema in the user message exactly
   (only those statement kinds, field names, and the `s<num>` sid format). Do
   **not** include a `contract`, `description`, `notes`, or any other top-level
   field: output exactly `program`, `version`, `entry`, `modules`.
2. a copy of the **same Rust program** with `cir_trace::ev("<tag>", "<sid>")`
   inserted according to these rules, which match the verifier's conformance
   checker:
   - `mutex_lock`, `semaphore_acquire`, `condvar_wait`, `channel_send`,
     `channel_recv`: emit `ev` **after the call returns** (a completed step);
   - `mutex_unlock`, `condvar_notify`/`condvar_notify_all`, `semaphore_release`,
     `spawn`, `scope`, `join`: emit `ev` at the statement;
   - use tag `t0` for main and `t<sid>_<i>` for the i-th member of a `scope`;
   - the program must call `cir_trace::finish()` once at the end of `main`;
   - standard-library only, single-file, must compile.

**Every statement object must carry a `"sid"` string of the form `s1`, `s2`, ...**
(unique within its function); a statement without `sid` is a format error.

Minimal worked shape:

```json
{"program": "mp", "version": "3.5.0", "entry": "main::main", "modules": [{
  "name": "main",
  "resources": [{"name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync"}],
  "functions": [
    {"name": "main", "kind": "normal", "body": [
      {"sid": "s1", "kind": "scope", "funcs": ["main::t1"]},
      {"sid": "s2", "kind": "return"}]},
    {"name": "t1", "kind": "normal", "form": "closure", "body": [
      {"sid": "s1", "kind": "mutex_lock", "resource": "m"},
      {"sid": "s2", "kind": "mutex_unlock", "resource": "m"},
      {"sid": "s3", "kind": "return"}]}]}]}
```

Output a single JSON object `{"cir": <CIR>, "rust": "<annotated source>"}` and
nothing else.
