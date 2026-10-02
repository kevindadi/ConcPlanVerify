mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: a (lock), b (lock)
    let a = Arc::new(Mutex::new_named("a_mutex0#131", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#169", ()));

    // Worker w1: takes a, then b — same order as w2 (R5)
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = cir_trace::spawn("w1#311", move || {
        let _ga = a1.lock().unwrap(); // waits if held (R6)
        let _gb = b1.lock().unwrap(); // waits if held (R6)
        // Critical section: w1 now holds BOTH a and b simultaneously (R2)
        // ... work ...
        // Guards drop here at end of scope: each mutex released exactly once (R3)
    });

    // Worker w2: takes a, then b — identical order (R5)
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = cir_trace::spawn("w2#776", move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // Critical section: w2 holds both a and b (R2)
        // ... work ...
    });

    // Main task started the group (R1) and waits for both workers (R8)
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Printed only after both workers have completed (R8, R9)
    println!("DONE done=1");
 cir_trace::finish();}
