use concir_sync::Semaphore;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;

fn worker(m: Arc<Semaphore>, c: Arc<AtomicU8>) {
    let _permit = m.acquire();
    let value = c.load(Ordering::SeqCst);
    if value < 1 {
        c.store(value + 1, Ordering::SeqCst);
    }
}

fn w1(m: Arc<Semaphore>, c: Arc<AtomicU8>) {
    worker(m, c);
}

fn w2(m: Arc<Semaphore>, c: Arc<AtomicU8>) {
    worker(m, c);
}

fn main() {
    let m = Semaphore::new(1);
    let c = Arc::new(AtomicU8::new(0)); // c ranges from 0 to 2.

    let first = {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        thread::spawn(move || w1(m, c))
    };

    let second = {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        thread::spawn(move || w2(m, c))
    };

    first.join().unwrap();
    second.join().unwrap();

    println!("DONE done={}", c.load(Ordering::SeqCst));
}
