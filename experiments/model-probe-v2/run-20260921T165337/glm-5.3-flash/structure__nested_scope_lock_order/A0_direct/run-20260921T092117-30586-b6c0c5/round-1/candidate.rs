use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Outer worker: starts a nested group of two inner tasks.
    let worker = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || {
            // Both inner tasks acquire the mutexes in the same global order
            // (A before B), so no circular wait can occur and every
            // interleaving terminates.
            let a1 = Arc::clone(&a);
            let b1 = Arc::clone(&b);
            let t1 = thread::spawn(move || {
                let _ga = a1.lock().unwrap();
                let _gb = b1.lock().unwrap();
            });

            let a2 = Arc::clone(&a);
            let b2 = Arc::clone(&b);
            let t2 = thread::spawn(move || {
                let _ga = a2.lock().unwrap();
                let _gb = b2.lock().unwrap();
            });

            t1.join().unwrap();
            t2.join().unwrap();
        })
    };

    worker.join().unwrap();
    println!("DONE done=1");
}
