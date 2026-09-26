mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

fn increment_c(c: &AtomicU32) {
    loop {
        let current = c.load(Ordering::Acquire);
        let next = current + 1;
        match c.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(_) => {}
        }
    }
}

fn main() { cir_trace::init();
    let _gate = Semaphore::new_named("_gate_semaphore0", 2);
    let c = Arc::new(AtomicU32::new(0));

    let c_w1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("increment_c", move || {
        increment_c(&c_w1);
    });

    let c_w2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("increment_c", move || {
        increment_c(&c_w2);
    });

    w1.join().expect("w1 join");
    w2.join().expect("w2 join");

    debug_assert_eq!(c.load(Ordering::Acquire), 2);
    println!("DONE done=1");
 cir_trace::finish();}
