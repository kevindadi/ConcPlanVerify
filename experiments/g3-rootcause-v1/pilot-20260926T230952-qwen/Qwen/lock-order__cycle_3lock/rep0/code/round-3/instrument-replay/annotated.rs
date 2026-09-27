mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = cir_trace::spawn("t1_handle", move || {
        // Lock a then b as per CIR: mutex_lock main::a; mutex_lock main::b
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Critical work (empty)
        // Unlock b then a as per CIR: mutex_unlock main::b; mutex_unlock main::a
        drop(_guard_b);
        drop(_guard_a);
    });

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let t2_handle = cir_trace::spawn("t2_handle", move || {
        // Lock b then c as per CIR: mutex_lock main::b; mutex_lock main::c
        let _guard_b = b2.lock().unwrap();
        let _guard_c = c2.lock().unwrap();
        // Critical work (empty)
        // Unlock c then b as per CIR: mutex_unlock main::c; mutex_unlock main::b
        drop(_guard_c);
        drop(_guard_b);
    });

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let t3_handle = cir_trace::spawn("t3_handle", move || {
        // Lock a then c as per CIR: mutex_lock main::a; mutex_lock main::c
        let _guard_a = a3.lock().unwrap();
        let _guard_c = c3.lock().unwrap();
        // Critical work (empty)
        // Unlock c then a as per CIR: mutex_unlock main::c; mutex_unlock main::a
        drop(_guard_c);
        drop(_guard_a);
    });

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
