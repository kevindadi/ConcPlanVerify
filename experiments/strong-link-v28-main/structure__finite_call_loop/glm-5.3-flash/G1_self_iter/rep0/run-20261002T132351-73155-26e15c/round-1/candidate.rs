//! Main task calls the auxiliary routine `helper`, then begins the same
//! call sequence again.
//!
//! Concurrency-defect review and repairs:
//!
//! - Defect (shared state): a naive version might use a shared counter or
//!   mutex so `main` can detect when `helper` finishes. That violates R2
//!   and creates contention. Repair: `helper` is fully self-contained and
//!   communicates completion only through its own `JoinHandle`, which is
//!   per-call and never shared between tasks.
//!
//! - Defect (call overlap): if `main` spawned `helper`'s work and moved on
//!   without waiting, the second call could start before the first
//!   finished, violating R3. Repair: `helper` joins its own task before
//!   returning, so each call runs to completion before the next begins.
//!
//! - Defect (non-termination): a busy-wait or lock-based completion check
//!   could deadlock or spin forever on some interleavings, violating R4.
//!   Repair: the only synchronization is an unconditional `join` on a task
//!   that performs no blocking operations and holds no locks, so every
//!   schedule terminates.

use std::thread;

/// Auxiliary routine (role: `helper`).
///
/// Entirely self-contained: it creates its own task, waits for that task
/// to finish, and returns. It shares no mutexes, counters, or other state
/// with the calling task (R2).
fn helper() {
    let handle = thread::spawn(|| {
        // Auxiliary work runs here. Nothing is shared with `main`;
        // no locks, counters, or flags are touched.
    });

    // R3: the auxiliary call runs to completion before the calling task
    // starts the next call. Joining unconditionally guarantees this, and
    // also guarantees termination on every schedule (R4): the spawned
    // task does no blocking and holds no resources, so the join cannot
    // deadlock.
    handle.join().expect("helper task panicked");
}

fn main() {
    // R1: call the auxiliary routine, then begin the same call sequence
    // again. Because `helper` joins internally, the second call starts
    // only after the first has fully completed (R3).
    helper();
    helper();

    // R5: print exactly this line, then exit.
    println!("DONE done=1");
}
