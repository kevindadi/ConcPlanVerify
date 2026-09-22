mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
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

fn worker(s: Arc<Semaphore>) {
    s.acquire();
    // perform work
    s.release();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(2));

    let mut handles = Vec::new();
    for _ in 0..3 {
        let s_clone = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            worker(s_clone);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
