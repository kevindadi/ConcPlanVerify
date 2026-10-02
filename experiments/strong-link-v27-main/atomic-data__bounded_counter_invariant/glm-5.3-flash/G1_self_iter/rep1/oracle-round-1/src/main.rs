mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource: c ranges over 0..=2, starts at 0.
// m is the mutual-exclusion lock protecting c.
type Shared = Arc<Mutex<u32>>;

// Worker role w1: add 1 to c while holding m.
fn w1(c: Shared) {
    let m = c.lock().unwrap();      // acquire m
    let v = *m;                     // read c under m
    debug_assert!(v <= 1, "c out of range: {}", v); // R5: stays in 0..=2
    *m = v + 1;                     // write c under m
    drop(m);                        // release m
}

// Worker role w2: add 1 to c while holding m.
fn w2(c: Shared) {
    let m = c.lock().unwrap();      // acquire m
    let v = *m;                     // read c under m
    debug_assert!(v <= 1, "c out of range: {}", v); // R5: stays in 0..=2
    *m = v + 1;                     // write c under m
    drop(m);                        // release m
}

fn main() { cir_trace::init();
    // R2: single shared counter, range 0..=2, starts at 0.
    let c: Shared = Arc::new(Mutex::new_named("res_mutex0#989", 0));

    // R1: supervising task launches the two workers.
    let h1 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w1#1108", move || w1(c))
    };
    let h2 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w2#1199", move || w2(c))
    };

    // R1/R6: wait for both workers; every interleaving terminates
    // because neither worker blocks except on m, which is always
    // released promptly.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Read final value under the lock (R4).
    let m = c.lock().unwrap();
    let final_c = *m;
    drop(m);

    // R7: done is the completion flag; both workers finished and the
    // counter reached its final value, so done == 1 on every schedule.
    let done = (final_c == 2) as u32;
    println!("DONE done={}", done);
 cir_trace::finish();}
