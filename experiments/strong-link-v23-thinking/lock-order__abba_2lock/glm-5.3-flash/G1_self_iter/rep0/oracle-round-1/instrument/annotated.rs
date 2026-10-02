mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: lock `a` and lock `b`.
    let a = Arc::new(Mutex::new_named("a_mutex0#135", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#173", ()));

    // Worker t1
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("handle_t1#279", move || {
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
    let handle_t2 = cir_trace::spawn("handle_t2#731", move || {
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
 cir_trace::finish();}
