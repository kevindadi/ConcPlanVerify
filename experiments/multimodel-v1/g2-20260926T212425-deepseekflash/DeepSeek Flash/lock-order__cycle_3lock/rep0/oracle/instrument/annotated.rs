mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // Three locks shared among the workers.
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));

    // A global ordering semaphore ensures no circular wait (deadlock-free).
    // Only one worker may attempt to acquire its locks at a time.
    let gate = Semaphore::new_named("gate_semaphore0", 1);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let g1 = Arc::clone(&gate);
    let t1 = cir_trace::spawn("t1", move || {
        let _permit = g1.acquire();
        let _la = a1.lock().unwrap();
        let _lb = b1.lock().unwrap();
        // critical work
    });

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let g2 = Arc::clone(&gate);
    let t2 = cir_trace::spawn("t2", move || {
        let _permit = g2.acquire();
        let _lb = b2.lock().unwrap();
        let _lc = c2.lock().unwrap();
        // critical work
    });

    let c3 = Arc::clone(&c);
    let a3 = Arc::clone(&a);
    let g3 = Arc::clone(&gate);
    let t3 = cir_trace::spawn("t3", move || {
        let _permit = g3.acquire();
        let _lc = c3.lock().unwrap();
        let _la = a3.lock().unwrap();
        // critical work
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
