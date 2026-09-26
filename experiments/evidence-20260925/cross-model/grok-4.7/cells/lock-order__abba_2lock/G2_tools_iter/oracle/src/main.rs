mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let t1_done = Arc::new(AtomicUsize::new(0));
    let t2_done = Arc::new(AtomicUsize::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_flag = Arc::clone(&t1_done);
    let t1 = cir_trace::spawn("t1", move || {
        let permit_a = a1.acquire();
        let permit_b = b1.acquire();
        t1_flag.store(1, Ordering::SeqCst);
        permit_b.release();
        permit_a.release();
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_flag = Arc::clone(&t2_done);
    let t2 = cir_trace::spawn("t2", move || {
        let permit_a = a2.acquire();
        let permit_b = b2.acquire();
        t2_flag.store(1, Ordering::SeqCst);
        permit_b.release();
        permit_a.release();
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        t1_done.load(Ordering::SeqCst),
        t2_done.load(Ordering::SeqCst)
    );
 cir_trace::finish();}
