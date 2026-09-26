mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));

    let m1 = Arc::clone(&m);
    let w1 = cir_trace::spawn("w1", move || {
        let mut guard = m1.lock().unwrap();
        *guard += 1;
    });

    let m2 = Arc::clone(&m);
    let w2 = cir_trace::spawn("w2", move || {
        let mut guard = m2.lock().unwrap();
        *guard += 1;
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = if *m.lock().unwrap() == 2 { 1 } else { 0 };
    println!("DONE done={}", done);
 cir_trace::finish();}
