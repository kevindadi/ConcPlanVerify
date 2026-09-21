mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let lock1 = Arc::new(Mutex::new_named("lock1_mutex0", ()));
    let lock2 = Arc::new(Mutex::new_named("lock2_mutex0", ()));
    let lock3 = Arc::new(Mutex::new_named("lock3_mutex0", ()));

    let l1a = Arc::clone(&lock1);
    let l2a = Arc::clone(&lock2);
    let l1b = Arc::clone(&lock1);
    let l3b = Arc::clone(&lock3);
    let l2c = Arc::clone(&lock2);
    let l3c = Arc::clone(&lock3);

    // To avoid deadlock (R5, R8, R9), impose a global lock ordering:
    // every worker acquires locks in the order lock1 < lock2 < lock3.
    // Worker 1 needs lock1 and lock2 -> acquire lock1 then lock2.
    // Worker 2 needs lock2 and lock3 -> acquire lock2 then lock3.
    // Worker 3 needs lock3 and lock1 -> acquire lock1 then lock3.
    let h1 = cir_trace::spawn("h1", move || {
        let _g1 = l1a.lock().unwrap();
        let _g2 = l2a.lock().unwrap();
        // critical work holding both locks
    });

    let h2 = cir_trace::spawn("h2", move || {
        let _g2 = l2c.lock().unwrap();
        let _g3 = l3c.lock().unwrap();
        // critical work holding both locks
    });

    let h3 = cir_trace::spawn("h3", move || {
        let _g1 = l1b.lock().unwrap();
        let _g3 = l3b.lock().unwrap();
        // critical work holding both locks
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
