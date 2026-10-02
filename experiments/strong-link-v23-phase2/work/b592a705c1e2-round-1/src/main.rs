mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#70", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#108", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("h1#190", move || {
        // t1: lock a, then lock b (same order as t2, so no deadlock)
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        let work: i32 = 1;
        work
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("h2#485", move || {
        // t2: lock a, then lock b (same order as t1, so no deadlock)
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        let work: i32 = 1;
        work
    });

    let t1 = h1.join().unwrap();
    let t2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", t1, t2);
 cir_trace::finish();}
