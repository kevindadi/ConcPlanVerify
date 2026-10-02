mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // `m` is the mutual-exclusion lock; the integer it protects is the
    // shared counter `c`. `c` is declared to range from 0 to 2 and
    // starts at 0 (R2).
    let m: Arc<Mutex<u8>> = Arc::new(Mutex::new_named("res_mutex0#268", 0));

    // Worker w1 (R1, R3, R4).
    let m_for_w1 = Arc::clone(&m);
    let w1 = cir_trace::spawn("w1#357", move || {
        // Acquire the lock m; c may only be read or written while it is held.
        let mut guard = m_for_w1.lock().expect("w1: lock m poisoned");
        let c_before = *guard; // read of c under the lock
        debug_assert!(c_before <= 1, "c out of declared range 0..=2");
        *guard = c_before + 1; // write of c under the lock: c += 1 (R3)
    });

    // Worker w2 (R1, R3, R4).
    let m_for_w2 = Arc::clone(&m);
    let w2 = cir_trace::spawn("w2#822", move || {
        // Acquire the lock m; c may only be read or written while it is held.
        let mut guard = m_for_w2.lock().expect("w2: lock m poisoned");
        let c_before = *guard; // read of c under the lock
        debug_assert!(c_before <= 1, "c out of declared range 0..=2");
        *guard = c_before + 1; // write of c under the lock: c += 1 (R3)
    });

    // The supervising task (main) waits for both workers to finish (R1, R6).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Completion flag: both workers have finished, so done = 1.
    // Print exactly the required line (R7).
    let done: u8 = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
