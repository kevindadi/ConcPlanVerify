mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() -> i32 {
    let tmp = 1 + 2;
    tmp
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute();
    *acc += 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute();
    *acc += 1;
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#353", 0));

    let w1_handle = {
        let m = Arc::clone(&m);
        crate::cir_trace::spawn("w1#425", move || w1(m))
    };
    let w2_handle = {
        let m = Arc::clone(&m);
        crate::cir_trace::spawn("w2#523", move || w2(m))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    let done = if *m.lock().unwrap() == 2 { 1 } else { 0 };
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
