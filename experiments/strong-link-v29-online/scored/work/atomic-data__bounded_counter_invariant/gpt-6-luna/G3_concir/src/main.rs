mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Counter {
    c: i32,
}

fn w1(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let current = guard.c;
    guard.c = current + 1;
}

fn w2(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let current = guard.c;
    guard.c = current + 1;
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#375", Counter { c: 0 }, __cir_obs_Counter));

    let m1 = Arc::clone(&m);
    let h1 = crate::cir_trace::spawn("w1#442", move || w1(m1));

    let m2 = Arc::clone(&m);
    let h2 = crate::cir_trace::spawn("w2#516", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}

fn __cir_obs_Counter(v: &Counter, r: &str) { crate::cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
