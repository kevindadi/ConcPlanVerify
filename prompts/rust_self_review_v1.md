# Rust self-review prompt (v1, arm A1)

You are reviewing a Rust program for concurrency defects: deadlock, lost wakeup,
data race, or a thread that can block forever.

- Inspect the program for concurrency defects and, if you find any, output the
  full corrected program.
- Do not rely on external tools. Reason about the interleavings yourself.
- Keep the same overall design intent; fix the defect minimally.
- If you are convinced the program has no concurrency defect, reply with exactly
  the single token `NO_ISSUES` and nothing else.
- Otherwise output only the full corrected Rust source inside a single ```rust
  code fence.

An external crate `concir_sync` is already linked; write `use concir_sync::Semaphore;`. Do not declare `mod concir_sync` and do not implement a semaphore yourself. Semaphore API: `Semaphore::new(n)` (returns an `Arc`), `acquire()` (returns a permit whose `Drop` releases), `try_acquire()`, `permit.release()` (consumes the permit to release early). There is no `Semaphore::release`; never release the same permit twice.
