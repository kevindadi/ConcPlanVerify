mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: a (lock), b (lock)
    let a = Arc::new(Mutex::new_named("a_mutex0#131", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#169", ()));

    // Worker t1: acquires locks in global order a -> b
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("handle_t1#314", move || {
        let _guard_a = a1.lock().unwrap(); // blocks until a is free (R4)
        let _guard_b = b1.lock().unwrap(); // blocks until b is free (R4)
        // Critical work: both locks held simultaneously (R3)
        // Guards drop here, releasing a then b (R7)
    });

    // Worker t2: acquires locks in the same global order a -> b
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle_t2 = cir_trace::spawn("handle_t2#753", move || {
        let _guard_a = a2.lock().unwrap(); // same order: no circular wait (R5)
        let _guard_b = b2.lock().unwrap();
        // Critical work: both locks held simultaneously (R3)
        // Guards drop here, releasing a then b (R7)
    });

    // Main thread starts both workers and finishes only after both finish (R6)
    handle_t1.join().expect("worker t1 panicked");
    handle_t2.join().expect("worker t2 panicked");

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
