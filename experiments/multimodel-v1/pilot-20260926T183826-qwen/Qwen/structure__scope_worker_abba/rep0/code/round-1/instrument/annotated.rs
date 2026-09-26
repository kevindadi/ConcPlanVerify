mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = cir_trace::spawn("w1", move || {
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Both held here; drop order is reverse of acquisition: b then a
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = cir_trace::spawn("w2", move || {
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();
        // Both held here; drop order is reverse of acquisition: b then a
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
