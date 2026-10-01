use std::sync::{Arc, Mutex};
use std::thread;

// A simple counting semaphore implementation to satisfy the specification.
// The specification requires a permit pool starting with exactly one permit.
struct Semaphore {
    permits: Arc<Mutex<usize>>,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Arc::new(Mutex::new(permits)),
        }
    }

    fn acquire(&self) {
        loop {
            let mut guard = self.permits.lock().unwrap();
            if *guard > 0 {
                *guard -= 1;
                return;
            }
            // Drop the lock before yielding to avoid holding it while waiting
            drop(guard);
            thread::yield_now();
        }
    }

    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
    }
}

fn main() {
    // R2: Shared counting permit pool begins with exactly one permit.
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    // R1: Start two worker threads.
    let handle1 = thread::spawn(move || {
        w1(s1);
    });

    let handle2 = thread::spawn(move || {
        w2(s2);
    });

    // R1: Wait for both workers to finish.
    handle1.join().unwrap();
    handle2.join().unwrap();

    // R7: Print exactly "DONE done=1" and exit.
    println!("DONE done=1");
}

fn w1(s: Arc<Semaphore>) {
    // R3 & R4: Acquire permit, perform work, release permit.
    // Each worker may acquire more than once, but must balance acquisitions with releases.
    // For simplicity and correctness, we model a single acquisition/release cycle per worker,
    // which satisfies "may acquire more than once" (it acquires once) and "release exactly as many times".
    
    s.acquire();
    // Perform work (no-op for this specification, just ensuring mutual exclusion)
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    // Same logic as w1.
    s.acquire();
    // Perform work
    s.release();
}
