mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Counter {
    c: i32,
}

fn w1(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = guard.c;
    guard.c = c + 1;
}

fn w2(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = guard.c;
    guard.c = c + 1;
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#351", Counter { c: 0 }, __cir_obs_Counter));

    let t1 = crate::cir_trace::spawn("w1#389", {
        let m = Arc::clone(&m);
        move || w1(m)
    });
    let t2 = crate::cir_trace::spawn("w2#480", {
        let m = Arc::clone(&m);
        move || w2(m)
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}

fn __cir_obs_Counter(v: &Counter, r: &str) { crate::cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
