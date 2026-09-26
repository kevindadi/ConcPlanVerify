mod cir_trace;
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

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let c = Arc::new(AtomicU8::new(0)); // c ranges from 0 to 2.

    let first = {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        cir_trace::spawn("w1", move || w1(m, c))
    };

    let second = {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        cir_trace::spawn("w2", move || w2(m, c))
    };

    first.join().unwrap();
    second.join().unwrap();

    println!("DONE done={}", c.load(Ordering::SeqCst));
 cir_trace::finish();}
