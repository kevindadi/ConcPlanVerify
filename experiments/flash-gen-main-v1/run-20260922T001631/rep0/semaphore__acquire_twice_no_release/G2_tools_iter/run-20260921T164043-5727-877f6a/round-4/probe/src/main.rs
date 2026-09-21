use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct PermitPool {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl PermitPool {
    fn new(permits: usize) -> Self {
        PermitPool {
            permits: Mutex::new(permits),
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
        let handle = thread::spawn(move || {
            pool.acquire();
            // work while holding permit
            pool.release();

            pool.acquire();
            // more work while holding permit
            pool.release();

            let mut d = done.lock().unwrap();
            *d += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
}
