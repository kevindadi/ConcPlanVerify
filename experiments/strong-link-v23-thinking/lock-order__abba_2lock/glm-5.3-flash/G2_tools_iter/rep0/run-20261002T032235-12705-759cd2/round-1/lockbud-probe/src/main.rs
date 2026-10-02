// Rust program: two worker threads (t1, t2) sharing two locks (a, b).
// Deadlock is prevented by a fixed global lock order: every worker
// acquires `a` before `b` and releases them in reverse order.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: lock `a` and lock `b`.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // ---- Worker t1 ----
    let a_for_t1 = Arc::clone(&a);
    let b_for_t1 = Arc::clone(&b);
    let handle_t1 = thread::spawn(move || {
        // Fixed order: acquire `a` first, then `b`.
        // lock() blocks while the lock is busy, then continues (R4).
        let guard_a = a_for_t1.lock().unwrap();
        let guard_b = b_for_t1.lock().unwrap();

        // Critical work: both locks are held at the same time (R3).
        let _ = (&*guard_a, &*guard_b);

        // Release each lock before the worker finishes (R7).
        drop(guard_b);
        drop(guard_a);
    });

    // ---- Worker t2 ----
    let a_for_t2 = Arc::clone(&a);
    let b_for_t2 = Arc::clone(&b);
    let handle_t2 = thread::spawn(move || {
        // Same fixed order: acquire `a` first, then `b`.
        // Because both workers use the same order, a circular wait
        // (each holding one lock and waiting for the other) is
        // impossible (R5), so every interleaving terminates (R8).
        let guard_a = a_for_t2.lock().unwrap();
        let guard_b = b_for_t2.lock().unwrap();

        // Critical work: both locks are held at the same time (R3).
        let _ = (&*guard_a, &*guard_b);

        // Release each lock before the worker finishes (R7).
        drop(guard_b);
        drop(guard_a);
    });

    // Main thread starts both workers and waits for both to finish (R6).
    let t1_ok = handle_t1.join().is_ok();
    let t2_ok = handle_t2.join().is_ok();

    // Exactly one line of output, then exit (R9).
    println!("DONE t1={} t2={}", t1_ok as u8, t2_ok as u8);
}
