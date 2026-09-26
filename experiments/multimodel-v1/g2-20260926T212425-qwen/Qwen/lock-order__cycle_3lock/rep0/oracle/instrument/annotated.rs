mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("t1", move || {
        // Acquire in global order: a then b
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Critical work
    });

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let t2 = cir_trace::spawn("t2", move || {
        // Acquire in global order: b then c
        let _guard_b = b2.lock().unwrap();
        let _guard_c = c2.lock().unwrap();
        // Critical work
    });

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let t3 = cir_trace::spawn("t3", move || {
        // Acquire in global order: a then c (since a < c)
        let _guard_a = a3.lock().unwrap();
        let _guard_c = c3.lock().unwrap();
        // Critical work
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
