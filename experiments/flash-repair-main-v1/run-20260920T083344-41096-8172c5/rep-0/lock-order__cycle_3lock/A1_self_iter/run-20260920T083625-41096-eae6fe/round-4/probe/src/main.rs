use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let mtx_c = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = thread::spawn(move || {
        let ga = a1.lock().unwrap();
        let gb = b1.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    let (b2, c2) = (Arc::clone(&mtx_b), Arc::clone(&mtx_c));
    let w2 = thread::spawn(move || {
        let gb = b2.lock().unwrap();
        let gc = c2.lock().unwrap();
        drop(gc);
        drop(gb);
    });

    let (c3, a3) = (Arc::clone(&mtx_c), Arc::clone(&mtx_a));
    let w3 = thread::spawn(move || {
        let ga = a3.lock().unwrap();
        let gc = c3.lock().unwrap();
        drop(gc);
        drop(ga);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
