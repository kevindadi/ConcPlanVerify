mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let lock1 = Arc::new(Mutex::new_named("lock1_mutex0", ()));
    let lock2 = Arc::new(Mutex::new_named("lock2_mutex0", ()));

    let l1a = Arc::clone(&lock1);
    let l2a = Arc::clone(&lock2);
    let l1b = Arc::clone(&lock1);
    let l2b = Arc::clone(&lock2);

    // Both workers acquire locks in the same global order to prevent deadlock.
    let t1 = cir_trace::spawn("t1", move || {
        let _g1 = l1a.lock().unwrap();
        let _g2 = l2a.lock().unwrap();
        // critical work
    });

    let t2 = cir_trace::spawn("t2", move || {
        let _g1 = l1b.lock().unwrap();
        let _g2 = l2b.lock().unwrap();
        // critical work
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
