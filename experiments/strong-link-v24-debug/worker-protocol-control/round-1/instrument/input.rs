use std::sync::{Arc, Mutex};

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = std::thread::spawn(move || {
        // t1: lock a, then lock b (same order as t2, so no deadlock)
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        let work: i32 = 1;
        work
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = std::thread::spawn(move || {
        // t2: lock a, then lock b (same order as t1, so no deadlock)
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        let work: i32 = 1;
        work
    });

    let t1 = h1.join().unwrap();
    let t2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", t1, t2);
}
