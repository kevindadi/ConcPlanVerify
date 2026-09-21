use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let sem = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new(0usize));

    let mut handles = Vec::new();

    for _role in 0..3 {
        for _activation in 0..2 {
            let sem = Arc::clone(&sem);
            let done = Arc::clone(&done);
            let handle = thread::spawn(move || {
                sem.acquire();
                // do work while holding the single permit
                let mut d = done.lock().unwrap();
                *d += 1;
                drop(d);
                sem.release();
            });
            handles.push(handle);
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
}
