mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct PermitPool {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl PermitPool {
    fn new(initial: usize) -> Self {
        PermitPool {
            permits: Mutex::new(initial),
            condvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.condvar.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.condvar.notify_one();
    }
}

fn worker(pool: Arc<PermitPool>, id: usize) {
    // Each worker acquires the permit multiple times, doing work while holding it.
    // The number of acquire/release pairs differs per worker to create varied interleavings.
    let rounds = if id == 0 { 3 } else { 2 };

    for _ in 0..rounds {
        pool.acquire();
        // Critical section: perform work while holding the permit.
        // Simulate work with a small computation (no sleep/yield).
        let mut acc: u64 = 0;
        for i in 0..1000u64 {
            acc = acc.wrapping_add(i.wrapping_mul(id as u64 + 1));
        }
        std::hint::black_box(acc);
        pool.release();
    }
}

fn main() { cir_trace::init();
    let pool = Arc::new(PermitPool::new(1));

    let mut handles = Vec::new();
    for id in 0..2 {
        let pool_clone = Arc::clone(&pool);
        handles.push(thread::spawn(move || worker(pool_clone, id)));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
