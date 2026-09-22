mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            count: Mutex::new(permits),
            condvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.condvar.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.condvar.notify_one();
    }
}

fn worker(s: Arc<Semaphore>, id: usize) {
    // Each worker acquires the permit multiple times, doing work while holding it,
    // and releases exactly as many times as it acquired.
    let acquisitions = if id == 1 { 3 } else { 2 };

    for _ in 0..acquisitions {
        s.acquire();
        // Critical section: perform work while holding the permit.
        // (No other worker can be here at the same time.)
        s.release();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = cir_trace::spawn("w1", move || worker(s1, 1));
    let w2 = cir_trace::spawn("w2", move || worker(s2, 2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
