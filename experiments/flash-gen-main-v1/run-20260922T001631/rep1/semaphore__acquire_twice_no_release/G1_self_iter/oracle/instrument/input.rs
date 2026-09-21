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
    let pool = Arc::new(PermitPool::new(1));
    let done = Arc::new(Mutex::new(0usize));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let pool = Arc::clone(&pool);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || {
            pool.acquire();
            // do work
            pool.release();

            pool.acquire();
            // do more work
            pool.release();

            let mut d = done.lock().unwrap();
            *d += 1;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
}
