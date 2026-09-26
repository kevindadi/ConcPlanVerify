mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));

    let m1 = Arc::clone(&m);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        // main::w1
        let mut guard = m1.lock().unwrap();
        *guard += 1;
    });

    let m2 = Arc::clone(&m);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        // main::w2
        let mut guard = m2.lock().unwrap();
        *guard += 1;
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
