use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn worker(c: Arc<AtomicUsize>) {
    loop {
        let current = c.load(Ordering::SeqCst);
        let next = current + 1;
        if c.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            break;
        }
    }
}

fn main() {
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let w1 = thread::spawn(move || worker(c1));

    let c2 = Arc::clone(&c);
    let w2 = thread::spawn(move || worker(c2));

    w1.join().unwrap();
    w2.join().unwrap();

    let done = c.load(Ordering::SeqCst);
    println!("DONE done={}", done);
}
