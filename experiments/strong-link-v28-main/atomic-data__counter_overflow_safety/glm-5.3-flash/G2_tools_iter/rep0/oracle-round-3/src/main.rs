mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

// R2: c is the shared counter, declared to range 0..=2, initial value 0.
// It is guarded by the mutex m (R4), so every read/write of c happens
// while m is held.

// Worker w1: increment c under m, but only if the result stays within
// the required upper limit of 1 (R3, R5).
fn w1(m: &Mutex<u8>) {
    {
        // Acquire m; the guard is released automatically at the end of
        // this block, on every path (no explicit drop needed).
        let mut c = m.lock().unwrap();
        if *c < 1 {
            *c += 1;
        }
    } // m released here
}

// Worker w2: identical behaviour to w1.
fn w2(m: &Mutex<u8>) {
    {
        let mut c = m.lock().unwrap();
        if *c < 1 {
            *c += 1;
        }
    } // m released here
}

fn main() { cir_trace::init();
    // m: the mutual-exclusion lock protecting c.
    let m: Mutex<u8> = Mutex::new_named("m_mutex0#883", 0);

    // R1: the supervising task (this scope) launches both worker threads
    // and waits for both of them to finish. Neither worker ever blocks on
    // the other (m is never held across a wait or I/O), so every schedule
    // and interleaving terminates (R6).
    thread::scope(|s| {
        let t1 = s.spawn(|| w1(&m));
        let t2 = s.spawn(|| w2(&m));

        t1.join().unwrap();
        t2.join().unwrap();
    });

    // Exactly one worker performed its increment, so c == 1 here (R5, R7).
    // Read the final value of c while holding m, then release m before
    // doing any I/O.
    let done = {
        let c = m.lock().unwrap();
        *c
    };

    println!("DONE done={}", done);
 cir_trace::finish();}
