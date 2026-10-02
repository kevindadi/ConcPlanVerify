use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: lock `a` and lock `b`.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Worker t1: acquires locks in the global order a -> b.
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || {
        let _guard_a = a1.lock().unwrap(); // blocks until `a` is free (R4)
        let _guard_b = b1.lock().unwrap(); // blocks until `b` is free (R4)

        // Critical work: both locks held simultaneously (R3).
        // Guards drop here at scope end, releasing a then b (R7).
    });

    // Worker t2: acquires locks in the SAME global order a -> b.
    // Consistent ordering makes deadlock impossible (R5, R8).
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || {
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();

        // Critical work: both locks held simultaneously (R3).
        // Guards drop here at scope end, releasing a then b (R7).
    });

    // Main thread starts both workers and waits for both to finish (R1, R6).
    t1.join().unwrap();
    t2.join().unwrap();

    // Exactly one output line, then exit (R9).
    println!("DONE t1=1 t2=1");
}
