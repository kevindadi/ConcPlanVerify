mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));

    let t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#223", move || {
            let _a = a.lock().unwrap();
            let _b = b.lock().unwrap();
            // critical work holding both locks
        })
    };

    let t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#481", move || {
            let _a = a.lock().unwrap();
            let _b = b.lock().unwrap();
            // critical work holding both locks
        })
    };

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
