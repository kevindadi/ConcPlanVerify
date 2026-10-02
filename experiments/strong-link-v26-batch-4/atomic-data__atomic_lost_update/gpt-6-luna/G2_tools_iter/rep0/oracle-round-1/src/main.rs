mod cir_trace;
use std::sync::{atomic::{AtomicUsize, Ordering};
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut current = c.load(Ordering::SeqCst);

    loop {
        match c.compare_exchange(
            current,
            current + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return,
            Err(actual) => current = actual,
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
    let handle1 = cir_trace::spawn("w1#626", move || w1(c1));

    let c2 = Arc::clone(&c);
    let handle2 = cir_trace::spawn("w2#705", move || w2(c2));

    handle1.join().expect("w1 panicked");
    handle2.join().expect("w2 panicked");

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
 cir_trace::finish();}
