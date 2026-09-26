mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute(n: i64) -> i64 {
    let tmp = n + 1;
    tmp
}

fn w1(m: Arc<Mutex<i64>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute(1);
    *acc = *acc + 1;
    drop(acc);
}

fn w2(m: Arc<Mutex<i64>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute(2);
    *acc = *acc + 1;
    drop(acc);
}

fn main() { cir_trace::init();
    // Shared state: mutex `m` protecting the variable `acc` (init 0).
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i64));

    let m_w1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1", move || w1(m_w1));

    let m_w2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2", move || w2(m_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
