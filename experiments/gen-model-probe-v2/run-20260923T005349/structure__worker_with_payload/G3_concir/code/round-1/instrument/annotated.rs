mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() {
    let x: i64 = 21 * 2;
    let y: i64 = x + 1;
    let _ = y;
}

fn w1(m: Arc<Mutex<i64>>) {
    let mut acc = m.lock().unwrap();
    compute();
    let t: i64 = *acc;
    let t: i64 = t + 1;
    *acc = t;
    drop(acc);
}

fn w2(m: Arc<Mutex<i64>>) {
    let mut acc = m.lock().unwrap();
    compute();
    let t: i64 = *acc;
    let t: i64 = t + 1;
    *acc = t;
    drop(acc);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i64));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let h1 = cir_trace::spawn("h1", move || w1(m1));
    let h2 = cir_trace::spawn("h2", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done: i64 = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
