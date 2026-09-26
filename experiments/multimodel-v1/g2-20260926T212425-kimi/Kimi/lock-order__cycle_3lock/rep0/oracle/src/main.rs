mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Three shared locks.
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));

    // Worker t1 needs locks a and b.
    // Acquire in global order a -> b.
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let t1 = cir_trace::spawn("t1", move || {
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Critical work while holding both a and b.
        // Guards drop here, releasing both locks.
    });

    // Worker t2 needs locks b and c.
    // Acquire in global order b -> c.
    let (b2, c2) = (Arc::clone(&b), Arc::clone(&c));
    let t2 = cir_trace::spawn("t2", move || {
        let _guard_b = b2.lock().unwrap();
        let _guard_c = c2.lock().unwrap();
        // Critical work while holding both b and c.
    });

    // Worker t3 needs locks c and a.
    // Acquire in global order a -> c (NOT c -> a) so that all workers
    // request locks in a consistent global order a < b < c. This makes
    // circular waiting impossible, so no deadlock can occur in any
    // interleaving.
    let (c3, a3) = (Arc::clone(&c), Arc::clone(&a));
    let t3 = cir_trace::spawn("t3", move || {
        let _guard_a = a3.lock().unwrap();
        let _guard_c = c3.lock().unwrap();
        // Critical work while holding both a and c.
    });

    // Main thread waits for all workers to finish.
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
