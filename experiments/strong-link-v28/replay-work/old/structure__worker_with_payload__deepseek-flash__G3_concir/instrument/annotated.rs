mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute(seed: i32) -> i32 {
    let mut tmp = seed;
    tmp = tmp + 1;
    tmp
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute(1);
    *acc = 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute(1);
    *acc = 1;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#384", 0));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let t1 = cir_trace::spawn("w1#466", move || w1(m1));
    let t2 = cir_trace::spawn("w2#510", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
