mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() {
    let mut tmp = 0;
    tmp = tmp + 1;
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    compute();
    *acc += 1;
    drop(acc);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    compute();
    *acc += 1;
    drop(acc);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#371", 0));

    let m1 = Arc::clone(&m);
    let t1 = crate::cir_trace::spawn("w1#423", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = crate::cir_trace::spawn("w2#497", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
