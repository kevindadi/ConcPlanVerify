mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let pool = Arc::new(PermitPool::new(2));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0usize));

    let mut handles = Vec::new();
    for _ in 0..3 {
        let pool = Arc::clone(&pool);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || {
            pool.acquire();
            // perform work
            pool.release();
            let mut d = done.lock().unwrap();
            *d += 1;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let d = done.lock().unwrap();
    if *d == 3 {
        println!("DONE done=1");
    }
 cir_trace::finish();}
