mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    if guard.c < 1 {
        guard.c += 1;
    }
    drop(guard);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    if guard.c < 1 {
        guard.c += 1;
    }
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#396", Shared { c: 0 }, __cir_obs_Shared));

    let m1 = Arc::clone(&m);
    let t1 = crate::cir_trace::spawn("w1#462", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = crate::cir_trace::spawn("w2#536", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = m.lock().unwrap().c;
    println!("DONE done={}", done);
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
