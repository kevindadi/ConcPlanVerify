mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let lock1 = Arc::new(Mutex::new_named("lock1_mutex0", ()));
    let lock2 = Arc::new(Mutex::new_named("lock2_mutex0", ()));
    let lock3 = Arc::new(Mutex::new_named("lock3_mutex0", ()));

    // To avoid deadlock (R5, R8, R9), impose a global ordering on the locks
    // and always acquire them in that order. Worker 3 needs lock3 and lock1,
    // so it acquires lock1 first, then lock3.
    let l1 = Arc::clone(&lock1);
    let l2 = Arc::clone(&lock2);
    let w1 = cir_trace::spawn("w1", move || {
        let _a = l1.lock().unwrap();
        let _b = l2.lock().unwrap();
        // critical work
    });

    let l2 = Arc::clone(&lock2);
    let l3 = Arc::clone(&lock3);
    let w2 = cir_trace::spawn("w2", move || {
        let _a = l2.lock().unwrap();
        let _b = l3.lock().unwrap();
        // critical work
    });

    let l1 = Arc::clone(&lock1);
    let l3 = Arc::clone(&lock3);
    let w3 = cir_trace::spawn("w3", move || {
        let _a = l1.lock().unwrap();
        let _b = l3.lock().unwrap();
        // critical work
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
