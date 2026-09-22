mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;

/// A binary semaphore modeling the single shared permit.
/// At most one activation may hold it at any moment; others block
/// on the condition variable until the permit is released.
struct Permit {
    held: Mutex<bool>, // true while some activation holds the permit
    freed: Condvar,    // signaled when the permit becomes available
}

impl Permit {
    fn new() -> Self {
        Permit {
            held: Mutex::new(false),
            freed: Condvar::new(),
        }
    }

    /// Block until the permit is available, then take it.
    fn acquire(&self) {
        let mut held = self.held.lock().unwrap();
        while *held {
            held = self.freed.wait(held).unwrap();
        }
        *held = true;
    }

    /// Release the permit and wake one waiting activation.
    fn release(&self) {
        let mut held = self.held.lock().unwrap();
        *held = false;
        self.freed.notify_one();
    }
}

const ROLES: usize = 3;
const ACTIVATIONS_PER_ROLE: usize = 2;

fn main() { cir_trace::init();
    let permit = Arc::new(Permit::new());
    let completed = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    // Start three worker roles, each with two concurrent activations.
    for role in 0..ROLES {
        for activation in 0..ACTIVATIONS_PER_ROLE {
            let permit = Arc::clone(&permit);
            let completed = Arc::clone(&completed);
            handles.push(thread::spawn(move || {
                // Wait until the single shared permit becomes available.
                permit.acquire();

                // Critical section: this activation holds the only permit.
                let mut work = 0usize;
                for i in 0..10_000usize {
                    work = work
                        .wrapping_add(i ^ (role * ACTIVATIONS_PER_ROLE + activation));
                }
                std::hint::black_box(work);
                completed.fetch_add(1, Ordering::SeqCst);

                // Release the permit so a waiting activation can proceed.
                permit.release();
            }));
        }
    }

    // Wait for every activation of every role to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    let done = if completed.load(Ordering::SeqCst) == ROLES * ACTIVATIONS_PER_ROLE {
        1
    } else {
        0
    };
    println!("DONE done={}", done);
 cir_trace::finish();}
