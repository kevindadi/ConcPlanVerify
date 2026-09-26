mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32)); // c is protected by m, init 0

    let m1 = Arc::clone(&m);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        // main::w1
        let mut guard = m1.lock().unwrap();
        let val = *guard;
        if val < 1 {
            *guard = val + 1;
        }
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        // main::w2
        let mut guard = m2.lock().unwrap();
        let val = *guard;
        if val < 1 {
            *guard = val + 1;
        }
        drop(guard);
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
