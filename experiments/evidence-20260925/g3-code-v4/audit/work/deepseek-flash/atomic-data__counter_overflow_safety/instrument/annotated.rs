mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let w1 = cir_trace::spawn("w1", move || {
        let mut guard = m1.lock().unwrap();
        guard.c = 1;
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let w2 = cir_trace::spawn("w2", move || {
        let mut guard = m2.lock().unwrap();
        guard.c = 1;
        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = m.lock().unwrap().c;
    println!("DONE done={}", done);
 cir_trace::finish();}
