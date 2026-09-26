mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b_kept = Arc::new(Mutex::new_named("b_kept_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b_kept);
    let t1 = cir_trace::spawn("t1", move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        drop(_gb);
        drop(_ga);
    });

    let b2 = Arc::clone(&b_kept);
    let c2 = Arc::clone(&c);
    let t2 = cir_trace::spawn("t2", move || {
        let _gb = b2.lock().unwrap();
        let _gc = c2.lock().unwrap();
        drop(_gc);
        drop(_gb);
    });

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let t3 = cir_trace::spawn("t3", move || {
        let _ga = a3.lock().unwrap();
        let _gc = c3.lock().unwrap();
        drop(_gc);
        drop(_ga);
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
