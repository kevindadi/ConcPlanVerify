use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct PermitPool {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl PermitPool {
    fn new(count: usize) -> Self {
        PermitPool {
            permits: Mutex::new(count),
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

fn main() {
    let pool = Arc::new(PermitPool::new(2));
    let done = Arc::new(Mutex::new(0usize));

    let mut handles = Vec::new();

    for _ in 0..3 {
        let pool = Arc::clone(&pool);
        let done = Arc::clone(&done);
        let handle = thread::spawn(move || {
            pool.acquire();
            // Perform work while holding the permit.
            {
                let mut d = done.lock().unwrap();
                *d += 1;
            }
            pool.release();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let done_count = *done.lock().unwrap();
    println!("DONE done={}", done_count);
}
