use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: two locks, `a` and `b`.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Worker t1: acquires locks in a fixed global order (a, then b).
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = thread::spawn(move || {
        // R4: lock() blocks until the lock is free, then continues.
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();

        // R3: both locks are held simultaneously here — critical work.
        // (No shared mutable state beyond the locks themselves is required.)

        // R7: release each lock before finishing (explicit drops).
        drop(_guard_b);
        drop(_guard_a);
    });

    // Worker t2: acquires the same locks in the SAME order (a, then b).
    // R5: identical acquisition order makes a circular wait impossible,
    // so neither worker can deadlock the other.
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle_t2 = thread::spawn(move || {
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();

        // R3: both locks held simultaneously — critical work.

        // R7: release each lock before finishing.
        drop(_guard_b);
        drop(_guard_a);
    });

    // R6: main starts both workers and waits for both to finish.
    // R8: every interleaving terminates — lock() always eventually
    // succeeds because both workers release their locks and use a
    // consistent lock order, so no deadlock or livelock is possible.
    handle_t1.join().unwrap();
    handle_t2.join().unwrap();

    // R9: exactly one line of output, then exit.
    println!("DONE t1=1 t2=1");
}
