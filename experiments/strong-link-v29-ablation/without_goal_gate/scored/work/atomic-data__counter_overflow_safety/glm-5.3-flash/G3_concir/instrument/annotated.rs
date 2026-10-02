mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    if *guard < 1 {
        *guard = *guard + 1;
    }
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    if *guard < 1 {
        *guard = *guard + 1;
    }
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#373", 0));

    let m1 = Arc::clone(&m);
    let t1 = crate::cir_trace::spawn("w1#425", move || w1(&m1));

    let m2 = Arc::clone(&m);
    let t2 = crate::cir_trace::spawn("w2#500", move || w2(&m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let c = m.lock().unwrap();
    println!("DONE done={}", *c);
 crate::cir_trace::finish();}
