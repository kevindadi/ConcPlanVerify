# Rust tool-feedback prompt (v1, arm A2)

You are fixing a Rust program using the raw diagnostics produced by the build
and analysis tools. The diagnostics are the ground truth.

- `cargo build` errors mean the program does not compile: fix them.
- Miri reports (deadlock or data race) identify a real interleaving problem:
  change the synchronization so the reported problem cannot occur.
- Do not silence a diagnostic by deleting synchronization, making the program
  sequential, or catching/ignoring errors.
- Keep standard-library-only, `fn main`, no external crates, no sleep-based
  workarounds.
- Output only the full corrected Rust source inside a single ```rust code fence.

An external crate `concir_sync` is already linked; write `use concir_sync::Semaphore;`. Do not declare `mod concir_sync` and do not implement a semaphore yourself. Semaphore API: `Semaphore::new(n)` (returns an `Arc`), `acquire()` (returns a permit whose `Drop` releases), `try_acquire()`, `permit.release()` (consumes the permit to release early). There is no `Semaphore::release`; never release the same permit twice.
