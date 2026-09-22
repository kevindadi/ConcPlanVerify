mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() -> u64 {
    // Sequential helper routine performing only local computation.
    let mut x: u64 = 0;
    for i in 0..1000u64 {
        x = x.wrapping_add(i.wrapping_mul(3).wrapping_add(1));
    }
    x
}

fn worker(m: Arc<Mutex<u64>>, done: Arc<Mutex<u64>>) {
    let local = compute();
    {
        let mut acc = m.lock().unwrap();
        *acc = acc.wrapping_add(local % 2);
    }
    {
        let mut d = done.lock().unwrap();
        *d += 1;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0u64));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0u64));

    let m1 = Arc::clone(&m);
    let d1 = Arc::clone(&done);
    let w1 = cir_trace::spawn("w1", move || worker(m1, d1));

    let m2 = Arc::clone(&m);
    let d2 = Arc::clone(&done);
    let w2 = cir_trace::spawn("w2", move || worker(m2, d2));

    w1.join().unwrap();
    w2.join().unwrap();

    let acc = *m.lock().unwrap();
    let d = *done.lock().unwrap();
    println!("DONE done={}", d);
    let _ = acc;
 cir_trace::finish();}
