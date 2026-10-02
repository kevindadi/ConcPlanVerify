use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut current = c.load(Ordering::Relaxed);

    loop {
        match c.compare_exchange(
            current,
            current + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
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

fn main() {
    let c = Arc::new(AtomicUsize::new(0));

    let h1 = {
        let c = Arc::clone(&c);
        thread::spawn(move || w1(c))
    };
    let h2 = {
        let c = Arc::clone(&c);
        thread::spawn(move || w2(c))
    };

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    assert_eq!(c.load(Ordering::Relaxed), 2);
    println!("DONE done=1");
}
