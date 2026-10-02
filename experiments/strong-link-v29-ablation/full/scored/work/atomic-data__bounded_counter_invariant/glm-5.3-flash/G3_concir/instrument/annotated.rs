mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: &Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let _ = guard.c;
    guard.c = guard.c + 1;
    drop(guard);
}

fn w2(m: &Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let _ = guard.c;
    guard.c = guard.c + 1;
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#396", Shared { c: 0 }, __cir_obs_Shared));

    let m1 = Arc::clone(&m);
    let h1 = crate::cir_trace::spawn("w1#462", move || {
        w1(&m1);
    });

    let m2 = Arc::clone(&m);
    let h2 = crate::cir_trace::spawn("w2#554", move || {
        w2(&m2);
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    let guard = m.lock().unwrap();
    println!("DONE done={}", guard.c);
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
