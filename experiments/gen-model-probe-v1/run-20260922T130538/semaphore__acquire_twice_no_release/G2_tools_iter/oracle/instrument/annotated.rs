mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// A counting semaphore: a pool of permits.
struct PermitPool {
    permits: Mutex<usize>,
    available: Condvar,
}

impl PermitPool {
    fn new(permits: usize) -> Self {
        PermitPool {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    /// Block until a permit is available, then take it.
    /// While waiting, the internal mutex is released, so the current
    /// permit holder can always make progress and release its permit.
    fn acquire(&self) {
        let mut guard = self.permits.lock().unwrap();
        while *guard == 0 {
            guard = self.available.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    /// Return a permit to the pool and wake one waiting worker.
    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
        self.available.notify_one();
    }
}

fn worker(pool: Arc<PermitPool>, rounds: usize) {
    for round in 0..rounds {
        pool.acquire();
        // Work is performed only while holding the permit, so the two
        // workers never work at the same time.
        let mut acc = 0u64;
        for i in 0..1000u64 {
            acc = acc.wrapping_add(i.wrapping_mul(round as u64 + 1));
        }
        std::hint::black_box(acc);
        // Release exactly once per acquire, on every path.
        pool.release();
    }
}

fn main() { cir_trace::init();
    // One shared pool with exactly one permit.
    let pool = Arc::new(PermitPool::new(1));

    let pool1 = Arc::clone(&pool);
    let pool2 = Arc::clone(&pool);

    // Supervising task launches two workers and waits for both.
    let h1 = cir_trace::spawn("h1", move || worker(pool1, 2));
    let h2 = cir_trace::spawn("h2", move || worker(pool2, 2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
