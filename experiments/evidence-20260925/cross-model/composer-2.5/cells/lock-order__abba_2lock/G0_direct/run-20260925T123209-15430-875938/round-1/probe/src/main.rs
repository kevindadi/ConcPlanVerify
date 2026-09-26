use concir_sync::Semaphore;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>, done: Arc<AtomicU32>) {
    let _perm_a = a.acquire();
    let _perm_b = b.acquire();
    done.store(1, Ordering::Relaxed);
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>, done: Arc<AtomicU32>) {
    let _perm_a = a.acquire();
    let _perm_b = b.acquire();
    done.store(1, Ordering::Relaxed);
}

fn main() {
    let a = Semaphore::new(1);
    let b = Semaphore::new(1);
    let t1_done = Arc::new(AtomicU32::new(0));
    let t2_done = Arc::new(AtomicU32::new(0));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&t1_done);
        thread::spawn(move || t1(a, b, done))
    };

    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&t2_done);
        thread::spawn(move || t2(a, b, done))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        t1_done.load(Ordering::Relaxed),
        t2_done.load(Ordering::Relaxed)
    );
}
