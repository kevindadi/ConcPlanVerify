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

## Library

An external crate `concir_sync` is already linked; write `use concir_sync::Semaphore;`. Do not declare `mod concir_sync` and do not implement a semaphore yourself. Semaphore API: `Semaphore::new(n)` (returns an `Arc`), `acquire()` (returns a permit whose `Drop` releases), `try_acquire()`, `release()`.

## Entity conventions

- Each CIR `Mutex`/`Condvar`/`Channel`/`Semaphore` resource corresponds to
  exactly one synchronization primitive in your program, under the same name.
  Do not introduce a synchronization primitive that the CIR does not have.
- A CIR `var`/`Atomic` is ordinary shared data: put it inside the `Mutex` that
  protects it (or keep it local) and access it under that lock. **Do not add a
  separate lock for a `var`.**
- Implement a CIR `condvar_wait` as `while !predicate { guard = cv.wait(guard) }`
  with the predicate guarded by the paired mutex. The release and reacquisition
  of the mutex are implicit in the wait.
- `main` does only what the CIR `main` does: start and join the named threads,
  then print the terminal line. After joining all workers, `main` may read
  shared state to print the terminal line; otherwise it must not access shared
  state the CIR `main` does not access.

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
