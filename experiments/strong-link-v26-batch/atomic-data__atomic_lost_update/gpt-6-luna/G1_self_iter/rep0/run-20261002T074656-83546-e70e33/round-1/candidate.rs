use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut old = c.load(Ordering::SeqCst);

    loop {
        match c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
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

fn main() {
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let h1 = thread::spawn(move || w1(c1));
    let h2 = thread::spawn(move || w2(c2));

    let r1 = h1.join();
    let r2 = h2.join();

    r1.expect("w1 panicked");
    r2.expect("w2 panicked");

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
}
