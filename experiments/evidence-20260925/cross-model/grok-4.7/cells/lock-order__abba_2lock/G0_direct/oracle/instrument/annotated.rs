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

    let a_for_t2 = a.clone();
    let b_for_t2 = b.clone();
    let t1_flag = Arc::clone(&t1_done);
    let t2_flag = Arc::clone(&t2_done);

    let t1 = cir_trace::spawn("t1", move || {
        let permit_a = a.acquire();
        let permit_b = b.acquire();
        t1_flag.store(1, Ordering::SeqCst);
        drop(permit_b);
        drop(permit_a);
    });

    let t2 = cir_trace::spawn("t2", move || {
        let a = a_for_t2;
        let b = b_for_t2;
        let permit_a = a.acquire();
        let permit_b = b.acquire();
        t2_flag.store(1, Ordering::SeqCst);
        drop(permit_b);
        drop(permit_a);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        t1_done.load(Ordering::SeqCst),
        t2_done.load(Ordering::SeqCst)
    );
 cir_trace::finish();}
