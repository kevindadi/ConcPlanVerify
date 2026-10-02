mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared locks: first pair (a, b), second pair (c, d)
    let a = Arc::new(Mutex::new_named("a_mutex0#146", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#184", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#222", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#260", ()));

    let mut handles = Vec::new();

    // t1 and t2: both need locks a and b.
    // They acquire them in the same relative sequence (a, then b),
    // so no deadlock is possible within the first pair.
    for _ in 0..2 {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            let _guard_a = a.lock().unwrap();
            let _guard_b = b.lock().unwrap();
            // Work while holding both locks of the first pair.
            // Guards are dropped (locks released) before the thread finishes.
        }));
    }

    // t3 and t4: both need locks c and d.
    // They acquire them in the same relative sequence (c, then d),
    // so no deadlock is possible within the second pair.
    for _ in 0..2 {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let _guard_c = c.lock().unwrap();
            let _guard_d = d.lock().unwrap();
            // Work while holding both locks of the second pair.
            // Guards are dropped (locks released) before the thread finishes.
        }));
    }

    // Main thread starts all four workers and waits for all of them.
    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
