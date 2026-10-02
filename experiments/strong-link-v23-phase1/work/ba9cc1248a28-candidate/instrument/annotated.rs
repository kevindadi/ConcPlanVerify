mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = cir_trace::spawn("t1_handle#214", move || {
        // Worker t1: acquire locks in consistent order (a then b) to prevent deadlock
        let _lock_a = a1.lock().unwrap();
        let _lock_b = b1.lock().unwrap();
        // Critical work happens while holding both locks
        drop(_lock_b);
        drop(_lock_a);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = cir_trace::spawn("t2_handle#600", move || {
        // Worker t2: acquire locks in same consistent order (a then b) to prevent deadlock
        let _lock_a = a2.lock().unwrap();
        let _lock_b = b2.lock().unwrap();
        // Critical work happens while holding both locks
        drop(_lock_b);
        drop(_lock_a);
    });

    // Main thread waits for both workers to finish
    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
