use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct PermitPool {
    permits: Mutex<usize>,
    available: Condvar,
}

impl PermitPool {
    fn new(initial: usize) -> Self {
        PermitPool {
            permits: Mutex::new(initial),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }
}

fn worker(pool: Arc<PermitPool>, id: usize) {
    // Each worker acquires the permit multiple times, releasing exactly as many times.
    let acquisitions = if id == 0 { 3 } else { 2 };

    for _ in 0..acquisitions {
        pool.acquire();
        // Perform work while holding the permit.
        // Simulate work with a small computation.
        let mut sum: u64 = 0;
        for i in 0..1000 {
            sum = sum.wrapping_add(i);
        }
        std::hint::black_box(sum);
        pool.release();
    }
}

fn main() {
    let pool = Arc::new(PermitPool::new(1));

    let mut handles = Vec::new();
    for id in 0..2 {
        let pool_clone = Arc::clone(&pool);
        handles.push(thread::spawn(move || worker(pool_clone, id)));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
}
