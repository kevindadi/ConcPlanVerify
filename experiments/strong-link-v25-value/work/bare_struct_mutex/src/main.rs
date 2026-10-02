mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// UNKNOWN control: the flag lives inside a Mutex<Struct>, not a primitive inner.
// No value event can be attributed, so the goal stays unsupported.
use std::sync::{Arc};
use std::thread;

struct S { ready: bool }

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#263", S { ready: false }));
    let m2 = Arc::clone(&m);
    let h = cir_trace::spawn("h#330", move || {
        let mut g = m2.lock().unwrap();
        g.ready = true;
    });
    h.join().unwrap();
    let _ = m.lock().unwrap().ready;
    println!("DONE ready=true");
 cir_trace::finish();}
