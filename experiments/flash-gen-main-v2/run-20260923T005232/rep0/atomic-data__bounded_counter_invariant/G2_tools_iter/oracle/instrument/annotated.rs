mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0u8));

    let w1 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w1", move || {
            let mut guard = m.lock().unwrap();
            *guard += 1;
        })
    };

    let w2 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w2", move || {
            let mut guard = m.lock().unwrap();
            *guard += 1;
        })
    };

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
