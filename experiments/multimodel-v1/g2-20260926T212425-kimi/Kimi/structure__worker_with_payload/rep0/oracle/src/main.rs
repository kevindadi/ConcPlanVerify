mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Sequential helper routine: performs only local computation.
fn compute() -> i32 {
    let mut sum: i32 = 0;
    for i in 0..1024 {
        sum = sum.wrapping_add(i * i);
    }
    sum
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap(); // take the shared mutex (waits if held)
    let _local = compute();          // local computation while holding m
    *acc = 1;                        // update shared variable while holding m
}                                    // guard drops here: m is released

fn w2(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    let _local = compute();
    *acc = 1;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32)); // m protects acc

    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1", move || w1(m1));

    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
