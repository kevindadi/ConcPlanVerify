mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // Declared range is 0..=2; the safety invariant keeps the value at most 1.
    let m = Arc::new(Mutex::new_named("m_mutex0", 0u8));
    let gate = Semaphore::new_named("gate_semaphore0", 1);

    let c1 = Arc::clone(&m);
    let g1 = Arc::clone(&gate);
    let w1 = cir_trace::spawn("w1", move || {
        let _permit = g1.acquire();
        let mut c = c1.lock().unwrap();
        if *c < 1 {
            *c += 1;
        }
    });

    let c2 = Arc::clone(&m);
    let g2 = Arc::clone(&gate);
    let w2 = cir_trace::spawn("w2", move || {
        let _permit = g2.acquire();
        let mut c = c2.lock().unwrap();
        if *c < 1 {
            *c += 1;
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={done}");
 cir_trace::finish();}
