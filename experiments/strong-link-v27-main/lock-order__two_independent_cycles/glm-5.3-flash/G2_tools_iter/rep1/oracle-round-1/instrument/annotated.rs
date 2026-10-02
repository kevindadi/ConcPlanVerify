mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared locks: first pair (a, b), second pair (c, d).
    let a = Arc::new(Mutex::new_named("a_mutex0#147", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#185", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#223", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#261", ()));

    // t1: needs both locks of the first pair (a then b).
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("t1#401", move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // Holds both locks of its pair while working.
        // Locks are released automatically when guards drop at scope end.
    });

    // t2: needs both locks of the first pair, same relative sequence (a then b).
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = cir_trace::spawn("t2#793", move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // Holds both locks of its pair while working.
    });

    // t3: needs both locks of the second pair (c then d).
    let c1 = Arc::clone(&c);
    let d1 = Arc::clone(&d);
    let t3 = cir_trace::spawn("t3#1087", move || {
        let _gc = c1.lock().unwrap();
        let _gd = d1.lock().unwrap();
        // Holds both locks of its pair while working.
    });

    // t4: needs both locks of the second pair, same relative sequence (c then d).
    let c2 = Arc::clone(&c);
    let d2 = Arc::clone(&d);
    let t4 = cir_trace::spawn("t4#1405", move || {
        let _gc = c2.lock().unwrap();
        let _gd = d2.lock().unwrap();
        // Holds both locks of its pair while working.
    });

    // Main thread starts all four workers and finishes only after all complete.
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
