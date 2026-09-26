use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));

    let handle = thread::spawn(move || {
        // outer worker starts nested group of two inner tasks x1 and x2
        let a1 = Arc::clone(&a);
        let b1 = Arc::clone(&b);
        let h1 = thread::spawn(move || {
            // x1: take mutex A then B (same order as x2)
            let _lock_a = a1.lock().unwrap();
            let _lock_b = b1.lock().unwrap();
            // holds both at same time
        });

        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);
        let h2 = thread::spawn(move || {
            // x2: take mutex A then B (same order as x1)
            let _lock_a = a2.lock().unwrap();
            let _lock_b = b2.lock().unwrap();
            // holds both at same time
        });

        // outer completes only after both inner tasks have finished
        h1.join().unwrap();
        h2.join().unwrap();
    });

    handle.join().unwrap();

    println!("DONE done=1");
}
