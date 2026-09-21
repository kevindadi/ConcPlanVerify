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

    fn count(&self) -> usize {
        *self.permits.lock().unwrap()
    }
}

fn main() { cir_trace::init();
    let pool = Arc::new(PermitPool::new(1));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let pool = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            pool.acquire();
            // Perform work while holding the permit.
            pool.release();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE permits={}", pool.count());
 cir_trace::finish();}
