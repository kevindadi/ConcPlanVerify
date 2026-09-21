mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// A simple counting semaphore built from Mutex + Condvar.
struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cond.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cond.notify_one();
    }
}

// Shared state: two locks and two permits.
struct Shared {
    lock_a: Mutex<()>,
    lock_b: Mutex<()>,
    permit_1: Semaphore,
    permit_2: Semaphore,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        lock_a: Mutex::new_named("shared_mutex0", ()),
        lock_b: Mutex::new_named("shared_mutex1", ()),
        permit_1: Semaphore::new(0),
        permit_2: Semaphore::new(0),
    });

    let done_a = Arc::new(Mutex::new_named("done_a_mutex0", false));
    let done_b = Arc::new(Mutex::new_named("done_b_mutex0", false));

    // Worker A
    let s_a = Arc::clone(&shared);
    let da = Arc::clone(&done_a);
    let worker_a = cir_trace::spawn("worker_a", move || {
        // Take first lock.
        let _ga = s_a.lock_a.lock().unwrap();
        // Signal that we have taken our first lock.
        s_a.permit_1.release();
        // Wait for the other worker to take its first lock.
        s_a.permit_2.acquire();
        // Now take second lock.
        let _gb = s_a.lock_b.lock().unwrap();
        // Critical section: both locks held.
        // Release both locks (drop guards).
        drop(_gb);
        drop(_ga);
        *da.lock().unwrap() = true;
    });

    // Worker B
    let s_b = Arc::clone(&shared);
    let db = Arc::clone(&done_b);
    let worker_b = cir_trace::spawn("worker_b", move || {
        // Take first lock.
        let _gb = s_b.lock_b.lock().unwrap();
        // Signal that we have taken our first lock.
        s_b.permit_2.release();
        // Wait for the other worker to take its first lock.
        s_b.permit_1.acquire();
        // Now take second lock.
        let _ga = s_b.lock_a.lock().unwrap();
        // Critical section: both locks held.
        // Release both locks (drop guards).
        drop(_ga);
        drop(_gb);
        *db.lock().unwrap() = true;
    });

    // Bystander task: keeps making progress without ever finishing on its own.
    let s_by = Arc::clone(&shared);
    let bystander = cir_trace::spawn("bystander", move || {
        // The bystander repeatedly acquires and releases the locks,
        // making progress but never finishing. It must not prevent
        // the workers from finishing.
        loop {
            {
                let _g = s_by.lock_a.lock().unwrap();
            }
            {
                let _g = s_by.lock_b.lock().unwrap();
            }
        }
    });

    // Main thread waits for both workers to finish.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Bystander never finishes; detach it by not joining.
    // But we must ensure main terminates. We can't join the bystander
    // because it never finishes. We simply drop the handle.
    drop(bystander);

    let a = *done_a.lock().unwrap();
    let b = *done_b.lock().unwrap();
    println!("DONE a={} b={}", a as u32, b as u32);
 cir_trace::finish();}
