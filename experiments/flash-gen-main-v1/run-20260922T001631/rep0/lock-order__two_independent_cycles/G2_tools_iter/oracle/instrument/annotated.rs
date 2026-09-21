mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let lock1 = Arc::new(Mutex::new_named("lock1_mutex0", ()));
    let lock2 = Arc::new(Mutex::new_named("lock2_mutex0", ()));
    let lock3 = Arc::new(Mutex::new_named("lock3_mutex0", ()));
    let lock4 = Arc::new(Mutex::new_named("lock4_mutex0", ()));

    let mut handles = Vec::new();

    // Workers 1 and 2 use the first pair (lock1, lock2) in the same order.
    for _ in 0..2 {
        let a = Arc::clone(&lock1);
        let b = Arc::clone(&lock2);
        handles.push(thread::spawn(move || {
            let _g1 = a.lock().unwrap();
            let _g2 = b.lock().unwrap();
            // hold both locks while working
        }));
    }

    // Workers 3 and 4 use the second pair (lock3, lock4) in the same order.
    for _ in 0..2 {
        let a = Arc::clone(&lock3);
        let b = Arc::clone(&lock4);
        handles.push(thread::spawn(move || {
            let _g1 = a.lock().unwrap();
            let _g2 = b.lock().unwrap();
            // hold both locks while working
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
