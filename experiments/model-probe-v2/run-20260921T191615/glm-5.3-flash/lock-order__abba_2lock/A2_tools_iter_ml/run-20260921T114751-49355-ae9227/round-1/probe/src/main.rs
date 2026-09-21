use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = thread::spawn(move || {
        // Lock in a consistent global order: A first, then B.
        let ga = a1.lock().unwrap();
        let gb = b1.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    let (a2, b2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w2 = thread::spawn(move || {
        // Same order: A first, then B. This removes the circular
        // wait that caused the deadlock.
        let ga = a2.lock().unwrap();
        let gb = b2.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE t1=1 t2=1");
}
