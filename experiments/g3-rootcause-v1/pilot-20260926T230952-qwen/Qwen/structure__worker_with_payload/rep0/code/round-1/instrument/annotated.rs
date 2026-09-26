mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn compute() -> i32 {
    let tmp = 1 + 1;
    tmp
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _ = compute();
    *guard += 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _ = compute();
    *guard += 1;
}

fn main() { cir_trace::init();
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0));

    let m1 = Arc::clone(&acc);
    let h1 = cir_trace::spawn("w1", move || {
        w1(m1);
    });

    let m2 = Arc::clone(&acc);
    let h2 = cir_trace::spawn("w2", move || {
        w2(m2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
