use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = thread::spawn(move || {
        let ga = a1.lock().unwrap(); // lock mtx_a
        let gb = b1.lock().unwrap(); // lock mtx_b
        drop(gb);
        drop(ga);
    });

    let (a2, b2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w2 = thread::spawn(move || {
        let ga = a2.lock().unwrap(); // lock mtx_a (same order as w1)
        let gb = b2.lock().unwrap(); // lock mtx_b
        drop(gb);
        drop(ga);
    });

    w1.join().unwrap();
    w2.join().unwrap();
}
