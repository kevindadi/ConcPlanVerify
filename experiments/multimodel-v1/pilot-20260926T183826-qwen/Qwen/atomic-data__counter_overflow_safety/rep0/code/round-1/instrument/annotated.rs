mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));
    let c = Arc::clone(&m); // The variable 'c' is protected by 'm', so we use the same mutex to guard access.
                            // In ConcIR, 'c' is a var protected by lock 'm'. We model this by putting 'c' inside 'm'.

    let m1 = Arc::clone(&m);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        let mut val_guard = m1.lock().unwrap();
        let val = *val_guard;
        if val < 1 {
            *val_guard = val + 1;
        }
        drop(val_guard);
    });

    let m2 = Arc::clone(&m);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        let mut val_guard = m2.lock().unwrap();
        let val = *val_guard;
        if val < 1 {
            *val_guard = val + 1;
        }
        drop(val_guard);
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    let final_val = *m.lock().unwrap();
    println!("DONE done={}", final_val);
 cir_trace::finish();}
