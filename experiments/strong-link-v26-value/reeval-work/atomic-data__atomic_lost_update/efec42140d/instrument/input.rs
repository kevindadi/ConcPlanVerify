use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: Arc<AtomicUsize>) {
    let mut current = c.load(Ordering::SeqCst);

    loop {
        match c.compare_exchange(
            current,
            current + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

fn w1(c: Arc<AtomicUsize>) {
    increment(c);
}

fn w2(c: Arc<AtomicUsize>) {
    increment(c);
}

fn main() {
    let c = Arc::new(AtomicUsize::new(0));

    let handle1 = thread::spawn({
        let c = Arc::clone(&c);
        move || w1(c)
    });
    let handle2 = thread::spawn({
        let c = Arc::clone(&c);
        move || w2(c)
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
}
