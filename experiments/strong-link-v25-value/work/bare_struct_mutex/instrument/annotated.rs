mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// UNKNOWN control: the flag lives inside a Mutex<Struct>, not a primitive inner.
// No value event can be attributed, so the goal stays unsupported.
use std::sync::{Arc};
use std::thread;

struct S { ready: bool }

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#263", S { ready: false }, __cir_obs_S));
    let m2 = Arc::clone(&m);
    let h = crate::cir_trace::spawn("h#330", move || {
        let mut g = m2.lock().unwrap();
        g.ready = true;
    });
    h.join().unwrap();
    let _ = m.lock().unwrap().ready;
    println!("DONE ready=true");
 crate::cir_trace::finish();}

fn __cir_obs_S(v: &S, r: &str) { crate::cir_trace::record_value(&format!("{}::ready", r), v.ready as i64); }
