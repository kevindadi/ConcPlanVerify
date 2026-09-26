mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared counter c (declared range 0..=2, starts at 0),
    // guarded by the mutual-exclusion lock m.
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));

    // Worker w1: adds exactly one to c while holding m.
    let m1 = Arc::clone(&m);
    let w1 = cir_trace::spawn("w1", move || {
        let mut c = m1.lock().unwrap();
        *c += 1;
    });

    // Worker w2: adds exactly one to c while holding m.
    let m2 = Arc::clone(&m);
    let w2 = cir_trace::spawn("w2", move || {
        let mut c = m2.lock().unwrap();
        *c += 1;
    });

    // Supervising task waits for both workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
