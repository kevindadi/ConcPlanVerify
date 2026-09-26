mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = cir_trace::spawn("t1_handle", move || {
        // t1: mutex_lock main::a; mutex_lock main::b; mutex_unlock main::b; mutex_unlock main::a
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Critical work happens here while both locks are held.
        // Drop guards in reverse order of acquisition to match CIR unlock sequence (b then a).
        drop(_guard_b);
        drop(_guard_a);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = cir_trace::spawn("t2_handle", move || {
        // t2: mutex_lock main::a; mutex_lock main::b; mutex_unlock main::b; mutex_unlock main::a
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();
        // Critical work happens here while both locks are held.
        // Drop guards in reverse order of acquisition to match CIR unlock sequence (b then a).
        drop(_guard_b);
        drop(_guard_a);
    });

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
