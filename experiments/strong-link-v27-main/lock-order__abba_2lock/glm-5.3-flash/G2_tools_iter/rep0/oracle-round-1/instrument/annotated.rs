mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: a (lock), b (lock)
    let a = Arc::new(Mutex::new_named("a_mutex0#131", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#169", ()));

    // Worker t1
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("handle_t1#275", move || {
        // Always acquire in the same order (a, then b) to prevent deadlock.
        // lock() blocks until the lock is free, satisfying the wait-and-continue rule.
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();

        // Critical work: both locks are held at the same time here.

        // Release each lock before finishing.
        drop(_guard_b);
        drop(_guard_a);
    });

    // Worker t2
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle_t2 = cir_trace::spawn("handle_t2#820", move || {
        // Same acquisition order (a, then b) as t1: no circular wait is possible.
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();

        // Critical work: both locks are held at the same time here.

        // Release each lock before finishing.
        drop(_guard_b);
        drop(_guard_a);
    });

    // Main thread starts both workers and finishes only after both have finished.
    handle_t1.join().unwrap();
    handle_t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
