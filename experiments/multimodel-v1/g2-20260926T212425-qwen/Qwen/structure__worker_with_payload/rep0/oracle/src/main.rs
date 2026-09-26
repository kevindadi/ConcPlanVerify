mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() -> i32 {
    // Sequential helper routine that performs only local computation.
    1
}

fn main() { cir_trace::init();
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0i32));

    let m1 = Arc::clone(&acc);
    let w1 = cir_trace::spawn("compute", move || {
        let mut guard = m1.lock().unwrap();
        let val = compute();
        *guard += val;
        drop(guard);
    });

    let m2 = Arc::clone(&acc);
    let w2 = cir_trace::spawn("compute", move || {
        let mut guard = m2.lock().unwrap();
        let val = compute();
        *guard += val;
        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done={}", *acc.lock().unwrap());
 cir_trace::finish();}
