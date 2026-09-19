# Rust -> CIR extraction prompt (v3, two fenced blocks)

You are given a Rust concurrent program and a machine schema. Reply with
**exactly two fenced blocks and nothing else**:

1. a ```json block containing a ConcIR model (only the schema's statement kinds,
   field names, and `program`/`version`/`entry`/`modules` at top level — no
   `contract`/`description`/`notes`). Statement `sid`s may be omitted if
   inconvenient; they are filled in deterministically from order.
2. a ```rust block containing the same program with
   `cir_trace::ev("<tag>", "<sid>")` inserted, using the **same sids as the CIR
   you just wrote** (write the CIR first, then reference its sids). Rules:
   - `mutex_lock`, `semaphore_acquire`, `condvar_wait`, `channel_send`,
     `channel_recv`: emit `ev` **before** the call (attempt);
   - `mutex_unlock`, `condvar_notify`/`condvar_notify_all`, `semaphore_release`,
     `spawn`, `scope`, `join`: emit `ev` at the statement;
   - tag `t0` for main, `t<sid>_<i>` for the i-th member of a `scope`;
   - the program must call `cir_trace::finish()` once at the end of `main`.

The CIR and the Rust must describe the same program; do not "fix" it.
