mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c += 1;
    }
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c += 1;
    }
    drop(c);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#323", 0));

    let m1 = Arc::clone(&m);
    let t1 = crate::cir_trace::spawn("w1#375", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = crate::cir_trace::spawn("w2#449", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done={}", *m.lock().unwrap());
 crate::cir_trace::finish();}
