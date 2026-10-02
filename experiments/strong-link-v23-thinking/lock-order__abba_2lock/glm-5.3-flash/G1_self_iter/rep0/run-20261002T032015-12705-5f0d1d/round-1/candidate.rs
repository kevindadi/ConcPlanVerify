use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: lock `a` and lock `b`.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Worker t1
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = thread::spawn(move || {
        // Global lock order: a, then b. Blocking lock waits if busy (R4).
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();

        // Critical work while holding BOTH locks (R3).
        // Guards drop here at scope end: both locks released before
        // the worker finishes (R7).
    });

    // Worker t2
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle_t2 = thread::spawn(move || {
        // Same global lock order: a, then b — no circular wait possible (R5, R8).
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();

        // Critical work while holding BOTH locks (R3).
    });

    // Main thread starts both workers and finishes only after both finish (R6).
    handle_t1.join().expect("worker t1 panicked");
    handle_t2.join().expect("worker t2 panicked");

    // R9: exactly this line, then exit.
    println!("DONE t1=1 t2=1");
}
