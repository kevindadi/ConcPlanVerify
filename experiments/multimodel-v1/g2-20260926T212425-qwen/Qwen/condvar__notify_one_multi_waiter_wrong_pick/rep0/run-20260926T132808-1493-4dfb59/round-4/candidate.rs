use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Shared state: a lock and a condition variable.
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    
    // g12 is used by waiters to signal they are ready to wait.
    // Initialized to 0 permits. Waiters will release (increment) it.
    // Notifier will acquire (decrement) it twice to ensure both waiters have signaled.
    let g12 = Semaphore::new(0);
    
    // gN is listed in entities but not strictly required for the core wake-up logic described.
    // We initialize it to satisfy the entity list requirement, though it may remain unused or be used for future extensions.
    let _gN = Semaphore::new(0);
    
    let pair_clone_w1 = Arc::clone(&pair);
    let pair_clone_w2 = Arc::clone(&pair);
    let pair_clone_n = Arc::clone(&pair);
    
    let g12_clone_w1 = Arc::clone(&g12);
    let g12_clone_w2 = Arc::clone(&g12);
    let g12_clone_n = Arc::clone(&g12);
    
    // Start Waiter 1
    let h1 = thread::spawn(move || {
        let (lock, cvar) = &*pair_clone_w1;
        let mut guard = lock.lock().unwrap();
        
        // Signal that this waiter is ready to wait (has acquired lock)
        // Release increments the semaphore count.
        g12_clone_w1.release();
        
        // Wait until notified
        while !*guard {
            guard = cvar.wait(guard).unwrap();
        }
        
        // Done waiting
    });
    
    // Start Waiter 2
    let h2 = thread::spawn(move || {
        let (lock, cvar) = &*pair_clone_w2;
        let mut guard = lock.lock().unwrap();
        
        // Signal that this waiter is ready to wait
        g12_clone_w2.release();
        
        // Wait until notified
        while !*guard {
            guard = cvar.wait(guard).unwrap();
        }
        
        // Done waiting
    });
    
    // Start Notifier
    let hn = thread::spawn(move || {
        // Ensure both waiters are ready (have released their permits into g12)
        // Acquire 2 permits. This blocks until 2 releases happen.
        let p1 = g12_clone_n.acquire();
        let p2 = g12_clone_n.acquire();
        
        // Now we know both waiters have entered the lock and signaled.
        // The notifier must take the lock before waking the waiters (R5).
        let (lock, cvar) = &*pair_clone_n;
        let mut guard = lock.lock().unwrap();
        
        // Set the condition to true so waiters can proceed
        *guard = true;
        
        // Wake every waiter that is still blocked (R3, R7)
        cvar.notify_all();
        
        // Drop guard to release lock (R5)
        drop(guard);
        
        // Permits p1, p2 dropped here, releasing them back to semaphore
    });
    
    // Join all threads
    h1.join().unwrap();
    h2.join().unwrap();
    hn.join().unwrap();
    
    println!("DONE waiters=0");
}
