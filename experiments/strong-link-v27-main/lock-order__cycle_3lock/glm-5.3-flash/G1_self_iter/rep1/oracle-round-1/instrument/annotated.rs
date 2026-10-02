mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared locks a, b, c (R2: each held by at most one worker at a time).
    let a = Arc::new(Mutex::new_named("a_mutex0#164", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#202", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#240", ()));

    // Global acquisition order: a < b < c. This breaks the circular wait,
    // so no schedule can deadlock (R5, R8, R9).

    // t1 needs a and b: acquires a, then b.
    let (la, lb) = (Arc::clone(&a), Arc::clone(&b));
    let t1 = cir_trace::spawn("t1#487", move || {
        let _g1 = la.lock().unwrap(); // a
        let _g2 = lb.lock().unwrap(); // b
        // Critical work with both locks held (R4).
        // Guards drop here, releasing both locks before the thread ends (R6).
    });

    // t2 needs b and c: acquires b, then c.
    let (lb, lc) = (Arc::clone(&b), Arc::clone(&c));
    let t2 = cir_trace::spawn("t2#848", move || {
        let _g1 = lb.lock().unwrap(); // b
        let _g2 = lc.lock().unwrap(); // c
        // Critical work with both locks held (R4).
    });

    // t3 needs c and a: acquires a, then c (respects global order a < c).
    let (la, lc) = (Arc::clone(&a), Arc::clone(&c));
    let t3 = cir_trace::spawn("t3#1160", move || {
        let _g1 = la.lock().unwrap(); // a
        let _g2 = lc.lock().unwrap(); // c
        // Critical work with both locks held (R4).
    });

    // Main starts all three workers and waits for all of them (R1, R7).
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    // Exactly one line of output, then exit (R10).
    println!("DONE done=1");
 cir_trace::finish();}
