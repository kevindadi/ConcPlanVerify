mod cir_trace;
use std::sync::Arc;
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A simple counting semaphore implemented with an atomic counter and spin-waiting.
struct Semaphore {
    count: AtomicUsize,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            count: AtomicUsize::new(permits),
        }
    }

    /// Acquire a permit, blocking until one is available.
    fn acquire(&self) {
        loop {
            // Try to decrement the count if it's > 0
            let current = self.count.load(Ordering::Acquire);
            if current > 0 {
                // Attempt CAS to decrement
                match self.count.compare_exchange_weak(
                    current,
                    current - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => return,
                    Err(_) => continue,
                }
            }
            // If no permits available, yield to avoid busy-waiting excessively
            thread::yield_now();
        }
    }

    /// Release a permit.
    fn release(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}

fn main() { cir_trace::init();
    // R2: Both worker threads share one counting permit pool that begins with exactly one permit.
    let s = Arc::new(Semaphore::new(1));

    // Track completion of workers
    let done_count = Arc::new(AtomicUsize::new(0));

    // R1: The program starts a supervising task that launches two worker threads and waits for both of them to finish.

    let s1 = Arc::clone(&s);
    let dc1 = Arc::clone(&done_count);
    let w1_handle = cir_trace::spawn("w1_handle#1656", move || {
        // R3: A worker must hold a permit while it performs its work
        // R4: Each worker may acquire the permit more than once, and on every path it must release the permit exactly as many times as it acquired it before it finishes.
        
        // For this problem, each worker acquires once and releases once.
        s1.acquire();
        
        // Perform work (simulated by doing nothing or minimal work)
        // The key constraint is mutual exclusion via the semaphore
        
        s1.release();
        
        // Mark this worker as done
        dc1.fetch_add(1, Ordering::SeqCst);
    });

    let s2 = Arc::clone(&s);
    let dc2 = Arc::clone(&done_count);
    let w2_handle = cir_trace::spawn("w2_handle#2389", move || {
        s2.acquire();
        
        // Perform work
        
        s2.release();
        
        // Mark this worker as done
        dc2.fetch_add(1, Ordering::SeqCst);
    });

    // Wait for both workers to finish
    w1_handle.join().expect("w1 panicked");
    w2_handle.join().expect("w2 panicked");

    // R7: The program must print exactly the line `DONE done=1` and then exit.
    // Since both workers completed, done_count should be 2, but the spec says "done=1"
    // Let me re-read: "print exactly the line `DONE done=1`"
    // This seems like a fixed output regardless of internal state.
    
    println!("DONE done=1");
 cir_trace::finish();}
