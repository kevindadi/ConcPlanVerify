mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    loop {
        let expected = c.load(Ordering::SeqCst);
        let desired = expected + 1;
        let old = c.compare_exchange(expected, desired, Ordering::SeqCst, Ordering::SeqCst);
        if old.is_ok() {
            return;
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("increment#469", move || {
        increment(&*c1);
    });

    let c2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("increment#569", move || {
        increment(&*c2);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
