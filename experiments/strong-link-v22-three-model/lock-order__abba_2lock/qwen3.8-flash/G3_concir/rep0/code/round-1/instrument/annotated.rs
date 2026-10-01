mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#70", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0#107", 0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = cir_trace::spawn("t1_handle#195", move || {
        // t1: mutex_lock main::a; mutex_lock main::b; mutex_unlock main::b; mutex_unlock main::a
        let _lock_a = a1.lock().unwrap();
        let _lock_b = b1.lock().unwrap();
        // Critical section done. Locks are released when guards go out of scope at end of block/function.
        // To strictly follow "release before finish" and ensure order if needed, we can explicitly drop or rely on scope.
        // The CIR says unlock b then unlock a. With RAII, dropping in reverse order of creation (b then a) is standard, 
        // but here we created lock_a then lock_b. So they will be dropped lock_b then lock_a. This matches the CIR order for unlocks?
        // CIR: unlock b, then unlock a.
        // Rust guard drop order: last acquired is first dropped.
        // Acquired: a, then b.
        // Dropped: b, then a.
        // This matches the CIR sequence s3 (unlock b) then s4 (unlock a).
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = cir_trace::spawn("t2_handle#1225", move || {
        // t2: mutex_lock main::a; mutex_lock main::b; mutex_unlock main::b; mutex_unlock main::a
        let _lock_a = a2.lock().unwrap();
        let _lock_b = b2.lock().unwrap();
        // Same logic as t1. Drop order: b then a. Matches CIR.
    });

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
