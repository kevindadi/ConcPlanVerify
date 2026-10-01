use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = thread::spawn(move || {
        // Worker t1: acquire locks in consistent order (a then b) to prevent deadlock
        let _lock_a = a1.lock().unwrap();
        let _lock_b = b1.lock().unwrap();
        // Critical work happens while holding both locks
        drop(_lock_b);
        drop(_lock_a);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = thread::spawn(move || {
        // Worker t2: acquire locks in same consistent order (a then b) to prevent deadlock
        let _lock_a = a2.lock().unwrap();
        let _lock_b = b2.lock().unwrap();
        // Critical work happens while holding both locks
        drop(_lock_b);
        drop(_lock_a);
    });

    // Main thread waits for both workers to finish
    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
}
