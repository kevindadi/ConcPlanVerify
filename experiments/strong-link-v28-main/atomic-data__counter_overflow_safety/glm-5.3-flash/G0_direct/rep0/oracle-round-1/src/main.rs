mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared counter c: declared to range from zero to two, starts at zero.
struct C(u8);

fn main() { cir_trace::init();
    // m is the mutual-exclusion lock guarding c.
    let m = Arc::new(Mutex::new_named("m_mutex0#225", C(0)));

    // Supervising task: launch worker w1.
    let m_w1 = Arc::clone(&m);
    let w1 = cir_trace::spawn("w1#325", move || {
        // Hold the lock whenever reading or writing c.
        let mut guard = m_w1.lock().unwrap();
        // Add one only if the value stays within the required upper limit (1).
        if guard.0 + 1 <= 1 {
            guard.0 += 1;
        }
        // Lock released when guard goes out of scope.
    });

    // Supervising task: launch worker w2.
    let m_w2 = Arc::clone(&m);
    let w2 = cir_trace::spawn("w2#748", move || {
        // Hold the lock whenever reading or writing c.
        let mut guard = m_w2.lock().unwrap();
        // Add one only if the value stays within the required upper limit (1).
        if guard.0 + 1 <= 1 {
            guard.0 += 1;
        }
        // Lock released when guard goes out of scope.
    });

    // Supervising task: wait for both workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();

    // In every reachable state c never exceeds 1, so the final value is 1.
    let done = m.lock().unwrap().0;
    println!("DONE done={}", done);
 cir_trace::finish();}
