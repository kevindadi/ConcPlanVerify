mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let tmp = *guard;
    if tmp < 1 {
        *guard = tmp + 1;
    }
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let tmp = *guard;
    if tmp < 1 {
        *guard = tmp + 1;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);
    let t1 = cir_trace::spawn("w1", move || w1(m1));
    let t2 = cir_trace::spawn("w2", move || w2(m2));
    t1.join().unwrap();
    t2.join().unwrap();
    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
