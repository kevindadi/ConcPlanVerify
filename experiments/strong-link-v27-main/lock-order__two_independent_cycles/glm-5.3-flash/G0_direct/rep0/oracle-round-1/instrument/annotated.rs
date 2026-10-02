mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// R1: one main thread + four worker threads (t1, t2, t3, t4)
// R2: four locks a, b (first pair) and c, d (second pair)
// R3: t1, t2 use pair (a, b); t3, t4 use pair (c, d)
// R4: each worker holds both locks of its pair while working
// R5: the two pairs use disjoint locks, so they are independent
// R6: t1 and t2 both take a then b (same relative sequence)
// R7: t3 and t4 both take c then d (same relative sequence)
// R8: consistent ordering within each pair prevents holding one lock
//     forever while waiting for the other (no deadlock)
// R9: guards drop (releasing locks) before each worker finishes
// R10: main starts all workers and joins them all
// R11: every interleaving terminates (lock ordering is consistent)
// R12: prints exactly `DONE done=1`

use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    // Shared locks: first pair (a, b), second pair (c, d).
    let a = Arc::new(cir_trace::sync::Mutex::new_named("a_mutex0#922", ()));
    let b = Arc::new(cir_trace::sync::Mutex::new_named("b_mutex0#971", ()));
    let c = Arc::new(cir_trace::sync::Mutex::new_named("c_mutex0#1020", ()));
    let d = Arc::new(cir_trace::sync::Mutex::new_named("d_mutex0#1069", ()));

    // t1: takes a then b.
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let t1 = cir_trace::spawn("t1#1173", move || {
        let _g1 = a1.lock().unwrap();
        let _g2 = b1.lock().unwrap();
        // work while holding both locks of the pair
    });

    // t2: takes a then b (same relative sequence as t1).
    let (a2, b2) = (Arc::clone(&a), Arc::clone(&b));
    let t2 = cir_trace::spawn("t2#1459", move || {
        let _g1 = a2.lock().unwrap();
        let _g2 = b2.lock().unwrap();
        // work while holding both locks of the pair
    });

    // t3: takes c then d.
    let (c3, d3) = (Arc::clone(&c), Arc::clone(&d));
    let t3 = cir_trace::spawn("t3#1714", move || {
        let _g1 = c3.lock().unwrap();
        let _g2 = d3.lock().unwrap();
        // work while holding both locks of the pair
    });

    // t4: takes c then d (same relative sequence as t3).
    let (c4, d4) = (Arc::clone(&c), Arc::clone(&d));
    let t4 = cir_trace::spawn("t4#2000", move || {
        let _g1 = c4.lock().unwrap();
        let _g2 = d4.lock().unwrap();
        // work while holding both locks of the pair
    });

    // Main thread waits for all four workers to finish.
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
