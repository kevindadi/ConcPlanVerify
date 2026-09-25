mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));
    let c = Arc::new(Mutex::new_named("c_mutex0", 0i32));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("w1", move || {
        let _guard = m1.lock().unwrap();
        let mut val = c1.lock().unwrap();
        *val += 1;
    });

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("w2", move || {
        let _guard = m2.lock().unwrap();
        let mut val = c2.lock().unwrap();
        *val += 1;
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *c.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
