mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Worker body shared by w1 and w2.
// Both workers acquire the mutexes in the SAME order: a, then b (R5),
// so no circular wait can form and every schedule terminates (R7).
// Mutex::lock() blocks until the mutex is free (R6), and a held mutex
// is exclusive, so at most one worker holds a given mutex (R4).
fn worker(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // Take mutex a.
    let _guard_a = a.lock().unwrap();

    // Take mutex b. At this point the worker holds both a and b (R2).
    let _guard_b = b.lock().unwrap();

    // ... work while holding both mutexes ...

    // Work finished: release each mutex exactly once (R3).
    drop(_guard_b);
    drop(_guard_a);
}

fn main() { cir_trace::init();
    // Shared resources: a and b (R1).
    let a = Arc::new(Mutex::new_named("a_mutex0#808", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#846", ()));

    // Main task starts the group of two workers, w1 and w2 (R1).
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = cir_trace::spawn("worker#994", move || worker(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = cir_trace::spawn("worker#1105", move || worker(a2, b2));

    // The group finishes only after both workers have completed (R8).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Exactly one output line, then exit (R9).
    println!("DONE done=1");
 cir_trace::finish();}
