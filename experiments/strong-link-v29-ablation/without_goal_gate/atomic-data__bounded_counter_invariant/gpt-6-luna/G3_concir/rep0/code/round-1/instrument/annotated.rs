mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let c = *guard;
    *guard = c + 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let c = *guard;
    *guard = c + 1;
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#307", 0));

    let w1_m = Arc::clone(&m);
    let t1 = crate::cir_trace::spawn("w1#361", move || w1(w1_m));

    let w2_m = Arc::clone(&m);
    let t2 = crate::cir_trace::spawn("w2#439", move || w2(w2_m));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
