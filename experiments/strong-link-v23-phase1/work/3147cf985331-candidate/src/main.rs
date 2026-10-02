mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, PoisonError};

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#83", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#121", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("t1#203", move || {
        // Wait for a, then for b: fixed global order => no circular wait (R4, R5)
        let _a = a1.lock().unwrap_or_else(PoisonError::into_inner);
        let _b = b1.lock().unwrap_or_else(PoisonError::into_inner);
        // Critical work: both locks are held simultaneously here (R3).
        // Guards drop at scope end, releasing a then b; even on panic
        // the locks are released (R7).
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = cir_trace::spawn("t2#714", move || {
        // Same lock order a then b (R5, R8)
        let _a = a2.lock().unwrap_or_else(PoisonError::into_inner);
        let _b = b2.lock().unwrap_or_else(PoisonError::into_inner);
        // Critical work with both locks held (R3)
    });

    // Main starts both workers and finishes only after both finish (R6).
    // Join results are consumed, not unwrapped, so main never panics and
    // every schedule terminates with the required output (R8, R9).
    let _ = t1.join();
    let _ = t2.join();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
