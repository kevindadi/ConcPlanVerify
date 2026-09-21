use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let sem = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new(0usize));

    let mut handles = Vec::new();

    for _ in 0..3 {
        let sem = Arc::clone(&sem);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || {
            for _ in 0..2 {
                sem.acquire();
                // do work while holding the permit
                sem.release();
            }
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
