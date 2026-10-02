mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    guard.c += 1;
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    guard.c += 1;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#300", Shared { c: 0 }, __cir_obs_Shared));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let t1 = cir_trace::spawn("w1#396", move || w1(m1));
    let t2 = cir_trace::spawn("w2#440", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
