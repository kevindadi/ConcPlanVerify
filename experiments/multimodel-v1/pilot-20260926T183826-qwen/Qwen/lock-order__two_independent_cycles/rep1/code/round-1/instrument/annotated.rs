mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("t1", move || {
        let _lock_a = a1.lock().unwrap();
        let _lock_b = b1.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_lock_b);
        drop(_lock_a);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = cir_trace::spawn("t2", move || {
        let _lock_a = a2.lock().unwrap();
        let _lock_b = b2.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_lock_b);
        drop(_lock_a);
    });

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let t3 = cir_trace::spawn("t3", move || {
        let _lock_c = c3.lock().unwrap();
        let _lock_d = d3.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_lock_d);
        drop(_lock_c);
    });

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let t4 = cir_trace::spawn("t4", move || {
        let _lock_c = c4.lock().unwrap();
        let _lock_d = d4.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_lock_d);
        drop(_lock_c);
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
