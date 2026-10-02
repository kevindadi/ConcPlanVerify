use std::sync::{Mutex, Arc};
use std::thread;

fn main() {
    // R2: Both worker threads share the same two locks.
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || {
            // R3: Each worker must hold both locks at the same time while it performs its critical work.
            // R5: To avoid deadlock (R8), we enforce a consistent lock ordering.
            // Always acquire 'a' first, then 'b'.
            
            // Critical section: modify values through the guards we already hold
            // Note: We cannot call a.lock() again because we already hold the guard _lock_a.
            // The MutexGuard derefs to &mut T, so we can use *guard += 1 directly.
            *(_lock_a) += 1;
            *(_lock_b) += 1;

            // R7: Every worker must release each lock it holds before it finishes.
            // The MutexGuard objects (_lock_a, _lock_b) will be dropped here, releasing the locks.
        })
    };

    let t2_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || {
            // R3: Each worker must hold both locks at the same time while it performs its critical work.
            // R5: Consistent lock ordering to prevent deadlock.
            // Always acquire 'a' first, then 'b'.
            let _lock_a = a.lock().unwrap();
            let _lock_b = b.lock().unwrap();

            // Critical section: modify values through the guards we already hold
            *(_lock_a) += 1;
            *(_lock_b) += 1;

            // R7: Every worker must release each lock it holds before it finishes.
            // The MutexGuard objects (_lock_a, _lock_b) will be dropped here, releasing the locks.
        })
    };

    // R6: The main thread starts both workers and only finishes after both workers have finished.
    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    // R9: The program must print exactly the line `DONE t1=1 t2=1` and then exit.
    println!("DONE t1=1 t2=1");
}
