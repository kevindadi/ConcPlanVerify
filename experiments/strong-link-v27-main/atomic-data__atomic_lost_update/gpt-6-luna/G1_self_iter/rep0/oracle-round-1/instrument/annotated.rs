mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut old = c.load(Ordering::Relaxed);

    loop {
        match c.compare_exchange(old, old + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(actual) => old = actual,
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
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = cir_trace::spawn("w1#581", move || w1(c1));
    let t2 = cir_trace::spawn("w2#625", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(c.load(Ordering::Relaxed), 2);
    println!("DONE done=1");
 cir_trace::finish();}
