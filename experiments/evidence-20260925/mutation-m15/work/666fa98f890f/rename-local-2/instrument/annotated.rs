mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1_kept = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = cir_trace::spawn("w1", move || {
        {
            let _ga = a1_kept.lock().unwrap();
            let _gb = b1.lock().unwrap();
        }
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = cir_trace::spawn("w2", move || {
        {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
