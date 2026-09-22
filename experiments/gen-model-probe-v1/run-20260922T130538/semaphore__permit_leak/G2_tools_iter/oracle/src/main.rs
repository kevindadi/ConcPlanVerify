mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::hint::black_box;
use std::sync::{Arc};
use std::thread;

/// A counting semaphore: a pool of permits protected by a mutex, with a
/// condition variable so waiters block until a permit is released.
struct PermitPool {
    permits: Mutex<usize>,
    available: Condvar,
}

impl PermitPool {
    fn new(n: usize) -> Self {
        PermitPool {
            permits: Mutex::new(n),
            available: Condvar::new(),
        }
    }

    /// Acquire one permit, blocking until one is available.
    fn acquire(&self) {
        let mut p = self.permits.lock().unwrap();
        // While this worker waits on the condvar, the mutex is released, so
        // the current permit holder can still lock it and release its permit.
        while *p == 0 {
            p = self.available.wait(p).unwrap();
        }
        *p -= 1;
    }

    /// Release one permit, waking one waiter if any.
    fn release(&self) {
        let mut p = self.permits.lock().unwrap();
        *p += 1;
        drop(p);
        self.available.notify_one();
    }

    fn count(&self) -> usize {
        *self.permits.lock().unwrap()
    }
}

fn worker(id: usize, pool: Arc<PermitPool>) {
    pool.acquire();
    // Perform work while holding the permit.
    let mut acc = 0u64;
    for k in 0..1000u64 {
        acc = acc.wrapping_add(id as u64 + k);
    }
    black_box(acc);
    pool.release();
}

fn main() { cir_trace::init();
    // Exactly one permit in the pool from the start.
    let pool = Arc::new(PermitPool::new(1));

    // The main thread is the supervisor: launch two workers and wait for both.
    let mut handles = Vec::new();
    for id in 0..2usize {
        let p = Arc::clone(&pool);
        handles.push(thread::spawn(move || worker(id, p)));
    }
    for h in handles {
        h.join().unwrap();
    }

    // After both workers released their permit, the pool is back to 1.
    println!("DONE permits={}", pool.count());
 cir_trace::finish();}
