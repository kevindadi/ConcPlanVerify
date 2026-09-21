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

    let done_a = Arc::new((Mutex::new_named("done_a_mutex0", false), Condvar::new_named("done_a_condvar0")));
    let done_b = Arc::new((Mutex::new_named("done_b_mutex0", false), Condvar::new_named("done_b_condvar0")));

    // Worker A
    let s_a = Arc::clone(&shared);
    let d_a = Arc::clone(&done_a);
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
        // Release locks (drop guards) before finishing.
        drop(_gb);
        drop(_ga);
        // Mark done.
        let (m, c) = &*d_a;
        let mut done = m.lock().unwrap();
        *done = true;
        c.notify_one();
    });

    // Worker B
    let s_b = Arc::clone(&shared);
    let d_b = Arc::clone(&done_b);
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
        // Release locks (drop guards) before finishing.
        drop(_ga);
        drop(_gb);
        // Mark done.
        let (m, c) = &*d_b;
        let mut done = m.lock().unwrap();
        *done = true;
        c.notify_one();
    });

    // Bystander task: keeps making progress without ever finishing.
    let s_by = Arc::clone(&shared);
    let bystander = cir_trace::spawn("bystander", move || {
        let mut counter: u64 = 0;
        loop {
            // Do some independent work that does not block workers.
            counter = counter.wrapping_add(1);
            // Occasionally touch the shared state in a non-blocking way.
            // We only try to lock briefly and release immediately.
            if let Ok(_g) = s_by.lock_a.try_lock() {
                // hold briefly, then release
            }
            if let Ok(_g) = s_by.lock_b.try_lock() {
                // hold briefly, then release
            }
            // Prevent the loop from being optimized away.
            if counter == u64::MAX {
                break;
            }
        }
    });

    // Wait for both workers to finish.
    {
        let (m, c) = &*done_a;
        let mut done = m.lock().unwrap();
        while !*done {
            done = c.wait(done).unwrap();
        }
    }
    {
        let (m, c) = &*done_b;
        let mut done = m.lock().unwrap();
        while !*done {
            done = c.wait(done).unwrap();
        }
    }

    // Join workers.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Bystander never finishes on its own; we detach it by not joining.
    // But we must ensure main terminates. We can't join it, so we just
    // let it be dropped. To avoid a warning, we explicitly forget it.
    std::mem::forget(bystander);

    println!("DONE a=1 b=1");
 cir_trace::finish();}
