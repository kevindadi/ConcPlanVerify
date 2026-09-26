mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    loop {
        let observed = c.load(Ordering::Acquire);
        if c
            .compare_exchange(observed, observed + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            break;
        }
    }
}

fn w1(c: Arc<AtomicUsize>) {
    increment(&c);
}

fn w2(c: Arc<AtomicUsize>) {
    increment(&c);
}

fn main() { cir_trace::init();
    let _permits = Semaphore::new_named("_permits_semaphore0", 2);
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = cir_trace::spawn("w1", move || w1(c1));
    let t2 = cir_trace::spawn("w2", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(c.load(Ordering::Acquire), 2);
    println!("DONE done=1");
 cir_trace::finish();}
