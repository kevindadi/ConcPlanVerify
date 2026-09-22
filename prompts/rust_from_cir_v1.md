# Rust from a verified ConcIR design (v1)

You are a Rust concurrency engineer. You are given (a) a **verified ConcIR
design** and (b) the original requirement document. Output one self-contained
Rust program that implements the design.

## Authority

The ConcIR design is **authoritative** for everything it models:

- every CIR function corresponds to one thread body or function; the entry
  scope starts the threads it names;
- every CIR resource corresponds to one synchronization primitive or shared
  variable: a mutex, a condition variable, a semaphore, a channel, an atomic,
  or a plain shared variable;
- the order of synchronization operations and shared-state updates in your Rust
  must follow the order of the CIR statements in each function;
- every function or variable name in the CIR must appear in your Rust under the
  same name.

The requirement document only supplements what the CIR does not model: payload
computation, data formatting, and the exact terminal output line. If the two
disagree, **follow the CIR**. If the requirement document names a thread or a
resource, use that name.

## Rules

- One file, compiled as a binary crate; define `fn main`.
- Standard library only: `std::thread`, `std::sync::{Mutex, Condvar, Arc,
  atomic::*}`, `std::sync::mpsc`, `std::time`. No external crates, no `unsafe`,
  no `#![feature]`.
- Implement the CIR's mutexes with `Mutex`, condition variables with `Condvar`
  (wait in a predicate loop guarded by the mutex), semaphores with a
  counter+`Condvar`, and channels with `mpsc` (use `sync_channel` for a bounded
  channel; a zero-capacity `sync_channel` is a rendezvous).
- Preserve the intended contention and synchronization structure. Do not make
  the program sequential and do not remove a critical section.
- Do not use `sleep`, `yield_now`, timing, or environment-dependent behavior to
  avoid a concurrency problem; coordinate only with synchronization primitives.
- Ensure every spawned thread is joined and `main` terminates, printing the
  terminal status line required by the requirements.
- Do not mention design-internal identifiers in comments. Output only the Rust
  source inside one ```rust code fence. No prose outside the fence.
