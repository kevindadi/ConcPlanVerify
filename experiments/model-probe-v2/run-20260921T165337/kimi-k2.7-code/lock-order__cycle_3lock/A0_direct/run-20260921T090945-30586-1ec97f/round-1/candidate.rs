use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let mtx_c = Arc::new(Mutex::new(()));

    // Worker 1: uses A and B (order A -> B).
    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
    });

    // Worker 2: uses B and C (order B -> C).
    let (b2, c2) = (Arc::clone(&mtx_b), Arc::clone(&mtx_c));
    let w2 = thread::spawn(move || {
        let _gb = b2.lock().unwrap();
        let _gc = c2.lock().unwrap();
    });

    // Worker 3: uses C and A, but acquire in global order A -> C to prevent deadlock.
    let (a3, c3) = (Arc::clone(&mtx_a), Arc::clone(&mtx_c));
    let w3 = thread::spawn(move || {
        let _ga = a3.lock().unwrap();
        let _gc = c3.lock().unwrap();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
