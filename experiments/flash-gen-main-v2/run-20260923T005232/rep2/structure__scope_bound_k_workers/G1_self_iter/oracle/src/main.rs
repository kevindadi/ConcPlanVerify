mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cond.notify_one();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0usize));

    let mut handles = Vec::new();

    for _ in 0..3 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || {
            // Each role may have up to two activations running at once.
            let mut inner = Vec::new();
            for _ in 0..2 {
                let s = Arc::clone(&s);
                let done = Arc::clone(&done);
                inner.push(thread::spawn(move || {
                    s.acquire();
                    // Do work while holding the single permit.
                    let mut d = done.lock().unwrap();
                    *d += 1;
                    drop(d);
                    s.release();
                }));
            }
            for h in inner {
                h.join().unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
 cir_trace::finish();}
