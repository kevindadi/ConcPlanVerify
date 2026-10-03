# Rust -> CIR extraction prompt (v1)

You are given a Rust concurrent program. Produce two things:

1. a **ConcIR** model of it that follows the machine schema provided in the user
   message exactly (use only those statement kinds, field names, and the `s<num>`
   sid format); and
2. a copy of the **same Rust program** with a call `cir_trace::ev("<tag>", "<sid>")`
   inserted immediately **before** every concurrency operation (lock/unlock,
   semaphore acquire/release, channel send/recv, condvar wait/notify, spawn,
   scope, join). The `sid` must be the sid of the matching statement in your CIR.
   Use tag `t0` for main and `t<sid>_<i>` for the i-th member of a `scope`.

Rules:

- Model the program faithfully; do not "fix" it. If it can deadlock, the model
  must be able to deadlock.
- The annotated Rust must compile as a standard-library-only binary and must run
  the same concurrency as the original.
- Output a single JSON object `{"cir": <CIR>, "rust": "<annotated source>"}` and
  nothing else.
