mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let gate = Semaphore::new_named("gate_semaphore0", 1);

    let n_t1 = Arc::new(AtomicU32::new(0));
    let n_t2 = Arc::new(AtomicU32::new(0));

    let a_1 = Arc::clone(&a);
    let b_1 = Arc::clone(&b);
    let gate_1 = Arc::clone(&gate);
    let n_t1_1 = Arc::clone(&n_t1);
    let t1 = cir_trace::spawn("t1", move || {
        let _only = gate_1.acquire();
        let _la = a_1.lock().unwrap();
        let _lb = b_1.lock().unwrap();
        n_t1_1.store(1, Ordering::Relaxed);
    });

    let a_2 = Arc::clone(&a);
    let b_2 = Arc::clone(&b);
    let gate_2 = Arc::clone(&gate);
    let n_t2_2 = Arc::clone(&n_t2);
    let t2 = cir_trace::spawn("t2", move || {
        let _only = gate_2.acquire();
        let _la = a_2.lock().unwrap();
        let _lb = b_2.lock().unwrap();
        n_t2_2.store(1, Ordering::Relaxed);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        n_t1.load(Ordering::Relaxed),
        n_t2.load(Ordering::Relaxed)
    );
 cir_trace::finish();}
