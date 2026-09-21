mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct PermitPool {
    permits: Mutex<usize>,
    available: Condvar,
}

impl PermitPool {
    fn new(count: usize) -> Self {
        PermitPool {
            permits: Mutex::new(count),
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

fn main() { cir_trace::init();
    let pool = Arc::new(PermitPool::new(2));
    let mut handles = Vec::new();

    for _ in 0..3 {
        let pool = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            pool.acquire();
            // perform work
            pool.release();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
