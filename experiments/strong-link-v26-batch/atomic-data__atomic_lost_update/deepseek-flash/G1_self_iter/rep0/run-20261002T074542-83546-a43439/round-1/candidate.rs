use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut cur = c.load(Ordering::SeqCst);
    loop {
        match c.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(actual) => cur = actual,
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
    let h1 = thread::spawn(move || w1(c1));

    let c2 = Arc::clone(&c);
    let h2 = thread::spawn(move || w2(c2));

    h1.join().unwrap();
    h2.join().unwrap();

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
}
