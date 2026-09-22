use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A counting semaphore (permit pool) built from a mutex and condition variable.
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

    /// Block until a permit is available, then take it.
    /// While waiting, the mutex is released so the current holder can
    /// reacquire it and release its permit.
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

    fn count(&self) -> usize {
        *self.permits.lock().unwrap()
    }
}

fn worker(id: u64, pool: Arc<PermitPool>) {
    // Acquire the single shared permit before working.
    pool.acquire();

    // Perform the work while holding the permit (mutual exclusion).
    let mut acc: u64 = 0;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i ^ id);
    }
    std::hint::black_box(acc);

    // Release the permit before finishing so the other worker can proceed.
    pool.release();
}

fn main() {
    // The shared counting permit pool starts with exactly one permit.
    let pool = Arc::new(PermitPool::new(1));

    // Supervising task: launches two workers and waits for both to finish.
    let supervisor_pool = Arc::clone(&pool);
    let supervisor = thread::spawn(move || {
        let mut handles = Vec::new();
        for id in 0..2u64 {
            let worker_pool = Arc::clone(&supervisor_pool);
            handles.push(thread::spawn(move || worker(id, worker_pool)));
        }
        for handle in handles {
            handle.join().unwrap();
        }
    });

    supervisor.join().unwrap();

    // Both workers released their permits, so the pool is back to one.
    println!("DONE permits={}", pool.count());
}
