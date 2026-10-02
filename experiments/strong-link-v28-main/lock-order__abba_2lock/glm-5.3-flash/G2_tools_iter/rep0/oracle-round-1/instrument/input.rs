// Design notes:
// - Two worker threads: t1 and t2. Two shared locks: a and b.
// - Deadlock (R5) is prevented by a global lock ordering: EVERY worker
//   acquires lock `a` before lock `b`, never the reverse. With a
//   consistent ordering, a cyclic wait (each worker holding one lock
//   and waiting for the other) is impossible, so every interleaving
//   terminates (R8).
// - Mutex::lock() blocks while the lock is busy and resumes once it is
//   free, satisfying R4.
// - Each worker holds both locks simultaneously during its critical
//   work (R3), and the lock guards are dropped (released) before the
//   worker finishes (R7).
// - The main thread spawns both workers and joins them, finishing only
//   after both have finished (R1, R6).

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared locks a and b (R2: shared by both workers; a Mutex can be
    // held by at most one thread at a time).
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Completion flags, read by the main thread after joining.
    let done_t1 = Arc::new(Mutex::new(0u32));
    let done_t2 = Arc::new(Mutex::new(0u32));

    // ---- Worker t1 ----
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let d1 = Arc::clone(&done_t1);
    let handle_t1 = thread::spawn(move || {
        // Always lock in the same order: a, then b (deadlock-free, R5).
        let _guard_a = a1.lock().unwrap(); // waits while busy (R4)
        let _guard_b = b1.lock().unwrap(); // waits while busy (R4)

        // Critical work: both locks are held at the same time here (R3).
        // (Locks a and b are exclusively held by this worker right now.)

        // Guards are dropped here at end of scope, releasing both
        // locks before the worker finishes (R7).
        *d1.lock().unwrap() = 1;
    });

    // ---- Worker t2 ----
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let d2 = Arc::clone(&done_t2);
    let handle_t2 = thread::spawn(move || {
        // Same lock order: a, then b (deadlock-free, R5).
        let _guard_a = a2.lock().unwrap(); // waits while busy (R4)
        let _guard_b = b2.lock().unwrap(); // waits while busy (R4)

        // Critical work: both locks are held at the same time here (R3).

        // Both locks released before finishing (R7).
        *d2.lock().unwrap() = 1;
    });

    // Main thread waits for both workers to finish (R6).
    handle_t1.join().unwrap();
    handle_t2.join().unwrap();

    let t1 = *done_t1.lock().unwrap();
    let t2 = *done_t2.lock().unwrap();

    // Exactly the required output line (R9).
    println!("DONE t1={} t2={}", t1, t2);
}
