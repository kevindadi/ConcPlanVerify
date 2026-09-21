use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            // R5: both workers take mutexes in the same order (A then B)
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
            // R2: both mutexes held simultaneously here
            // R3: guards released when they go out of scope
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
