mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let lock_a = Arc::new(Mutex::new_named("lock_a_mutex0", ()));
    let lock_b = Arc::new(Mutex::new_named("lock_b_mutex0", ()));

    let a1 = Arc::clone(&lock_a);
    let b1 = Arc::clone(&lock_b);
    let t1 = cir_trace::spawn("t1", move || {
        // Acquire both locks in a consistent global order to avoid deadlock.
        let _g1 = a1.lock().unwrap();
        let _g2 = b1.lock().unwrap();
        // Critical work while holding both locks.
    });

    let a2 = Arc::clone(&lock_a);
    let b2 = Arc::clone(&lock_b);
    let t2 = cir_trace::spawn("t2", move || {
        // Same acquisition order as t1.
        let _g1 = a2.lock().unwrap();
        let _g2 = b2.lock().unwrap();
        // Critical work while holding both locks.
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
