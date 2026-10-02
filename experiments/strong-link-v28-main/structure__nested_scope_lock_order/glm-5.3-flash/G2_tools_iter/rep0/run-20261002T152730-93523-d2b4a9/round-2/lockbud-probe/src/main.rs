use std::sync::{Arc, Mutex};
use std::thread;

// Shared resources: a (lock), b (lock).
// Both inner tasks acquire them in the SAME order: a, then b (R4).
// Because the acquisition order is consistent, no wait cycle can form,
// so every schedule and interleaving terminates (R6).

// Role: x1 / x2 (inner tasks). Each acquires a, then b, and at the
// point between the two acquisitions-and-release it holds both a and b
// simultaneously (R3).
fn inner_task(name: &'static str, a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // Acquire a first...
    let _guard_a = a.lock().expect("mutex a poisoned");
    // ...then b. Both inner tasks follow this same order.
    let _guard_b = b.lock().expect("mutex b poisoned");

    // At this point the task holds both a and b at the same time (R3).
    // (No output here: the program must print exactly one line, R7.)

    // Guards drop here, releasing b then a, in reverse acquisition order.
    drop(_guard_b);
    drop(_guard_a);
    let _ = name; // name kept for role identification only
}

// Role: outer. Starts the nested group of two inner tasks (R2) and
// completes only after both inner tasks have finished (R5).
fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || inner_task("x1", a1, b1));
    let h2 = thread::spawn(move || inner_task("x2", a, b));

    // Join both inner tasks; outer completes only after they finish (R5).
    h1.join().expect("x1 panicked");
    h2.join().expect("x2 panicked");
}

// Role: main task. Starts one outer worker (R1).
fn main() {
    let a = Arc::new(Mutex::new(())); // resource: a
    let b = Arc::new(Mutex::new(())); // resource: b

    let outer_handle = thread::spawn(move || outer(a, b));

    // Main task waits for the outer worker to complete.
    outer_handle.join().expect("outer panicked");

    // Exactly one line of output, then exit (R7).
    println!("DONE done=1");
}
