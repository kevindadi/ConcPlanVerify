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

fn main() {
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = thread::spawn(move || w1(c1));
    let t2 = thread::spawn(move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(c.load(Ordering::Relaxed), 2);
    println!("DONE done=1");
}
