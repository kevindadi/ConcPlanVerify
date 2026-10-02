mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut observed = c.load(Ordering::Relaxed);

    loop {
        match c.compare_exchange(
            observed,
            observed + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(actual) => observed = actual,
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c_w1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("increment#530", move || increment(&c_w1));

    let c_w2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("increment#616", move || increment(&c_w2));

    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    assert_eq!(c.load(Ordering::Relaxed), 2);
    println!("DONE done=1");
 cir_trace::finish();}
