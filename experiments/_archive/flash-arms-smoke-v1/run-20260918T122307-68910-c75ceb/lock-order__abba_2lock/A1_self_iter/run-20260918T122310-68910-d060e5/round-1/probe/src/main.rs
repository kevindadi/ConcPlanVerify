use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let worker1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // work inside critical section
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let worker2 = thread::spawn(move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // work inside critical section
    });

    worker1.join().unwrap();
    worker2.join().unwrap();
}
