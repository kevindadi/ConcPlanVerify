use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A counting permit pool (semaphore) built from a mutex and a condvar.
/// Waiting on the condvar releases the mutex, so permit holders can always
/// reacquire the lock to release their permit (R5).
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
    fn acquire(&self) {
        let mut guard = self.permits.lock().unwrap();
        while *guard == 0 {
            // Atomically releases the mutex while waiting, allowing
            // holders to enter `release`.
            guard = self.available.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    /// Return a permit and wake one waiting worker, if any.
    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
        self.available.notify_one();
    }
}

fn worker(pool: Arc<PermitPool>) {
    pool.acquire();

    // Perform work while holding the permit.
    let mut acc: u64 = 0;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i).wrapping_mul(3);
    }
    std::hint::black_box(acc);

    pool.release();
}

fn main() {
    // Supervising task: launches three workers and waits for all of them.
    let supervisor = thread::spawn(|| {
        let pool = Arc::new(PermitPool::new(2));

        let mut handles = Vec::new();
        for _ in 0..3 {
            let pool = Arc::clone(&pool);
            handles.push(thread::spawn(move || worker(pool)));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    supervisor.join().unwrap();
    println!("DONE done=1");
}
