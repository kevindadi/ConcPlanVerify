use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));

    let t1 = Arc::new(AtomicUsize::new(0));
    let t2 = Arc::new(AtomicUsize::new(0));

    let (a1, b1, done1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b), Arc::clone(&t1));
    let w1 = thread::spawn(move || {
        let ga = a1.lock().unwrap();
        let gb = b1.lock().unwrap();
        drop(gb);
        drop(ga);
        done1.store(1, Ordering::SeqCst);
    });

    let (a2, b2, done2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b), Arc::clone(&t2));
    let w2 = thread::spawn(move || {
        let ga = a2.lock().unwrap();
        let gb = b2.lock().unwrap();
        drop(gb);
        drop(ga);
        done2.store(1, Ordering::SeqCst);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        t1.load(Ordering::SeqCst),
        t2.load(Ordering::SeqCst)
    );
}
