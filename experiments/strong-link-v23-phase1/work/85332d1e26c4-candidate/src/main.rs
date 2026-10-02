mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R2: Both worker threads share the same two locks.
    let a = Arc::new(Mutex::new_named("a_mutex0#144", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0#181", 0));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1_handle#285", move || {
            // R3: Each worker must hold both locks at the same time while it performs its critical work.
            // R5: To avoid deadlock (R8), we enforce a consistent lock ordering.
            // Always acquire 'a' first, then 'b'.
            let _lock_a = a.lock().unwrap();
            let _lock_b = b.lock().unwrap();

            // Critical section
            *a.lock().unwrap() += 1;
            *b.lock().unwrap() += 1;

            // R7: Every worker must release each lock it holds before it finishes.
            // The MutexGuard objects (_lock_a, _lock_b) will be dropped here, releasing the locks.
        })
    };

    let t2_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2_handle#1043", move || {
            // R3: Each worker must hold both locks at the same time while it performs its critical work.
            // R5: Consistent lock ordering to prevent deadlock.
            // Always acquire 'a' first, then 'b'.
            let _lock_a = a.lock().unwrap();
            let _lock_b = b.lock().unwrap();

            // Critical section
            *a.lock().unwrap() += 1;
            *b.lock().unwrap() += 1;

            // R7: Every worker must release each lock it holds before it finishes.
            // The MutexGuard objects (_lock_a, _lock_b) will be dropped here, releasing the locks.
        })
    };

    // R6: The main thread starts both workers and only finishes after both workers have finished.
    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    // R9: The program must print exactly the line `DONE t1=1 t2=1` and then exit.
    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
