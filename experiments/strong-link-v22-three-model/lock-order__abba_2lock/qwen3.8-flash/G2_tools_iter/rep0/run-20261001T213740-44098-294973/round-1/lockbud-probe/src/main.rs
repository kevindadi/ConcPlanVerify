use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared locks a and b
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Counter to track how many workers have finished their critical section
    let done_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Worker t1
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let dc1 = Arc::clone(&done_count);
    let handle1 = thread::spawn(move || {
        // To avoid deadlock, we need a consistent lock ordering.
        // Let's say both workers always acquire lock 'a' first, then lock 'b'.
        
        // Acquire lock a
        let _guard_a = a1.lock().unwrap();
        // Acquire lock b while holding a
        let _guard_b = b1.lock().unwrap();
        
        // Critical work is done here (just holding the locks)
        // After this block, guards are dropped, releasing locks in reverse order: b then a
        
        // Mark completion
        dc1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });

    // Worker t2
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let dc2 = Arc::clone(&done_count);
    let handle2 = thread::spawn(move || {
        // Same lock ordering as t1: acquire a first, then b
        
        // Acquire lock a
        let _guard_a = a2.lock().unwrap();
        // Acquire lock b while holding a
        let _guard_b = b2.lock().unwrap();
        
        // Critical work is done here (just holding the locks)
        // After this block, guards are dropped, releasing locks in reverse order: b then a
        
        // Mark completion
        dc2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });

    // Wait for both workers to finish
    handle1.join().unwrap();
    handle2.join().unwrap();

    // Verify that both workers completed
    let count = done_count.load(std::sync::atomic::Ordering::SeqCst);
    
    if count == 2 {
        println!("DONE t1=1 t2=1");
    } else {
        panic!("Unexpected state");
    }
}
