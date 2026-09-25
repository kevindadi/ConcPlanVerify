mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("t1", move || {
        // Acquire locks in a consistent global order to avoid deadlock.
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // critical work
        // locks released here
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = cir_trace::spawn("t2", move || {
        // Same order as t1.
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // critical work
        // locks released here
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
