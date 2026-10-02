mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#70", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#108", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("t1#190", move || {
        let _a = a1.lock().unwrap();
        let _b = b1.lock().unwrap();
        // critical work with both locks held
        // always acquire a then b, release automatically in reverse order
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = cir_trace::spawn("t2#494", move || {
        let _a = a2.lock().unwrap();
        let _b = b2.lock().unwrap();
        // same lock order a then b; no deadlock possible
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
