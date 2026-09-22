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

fn worker(s: Arc<Semaphore>, done: Arc<Mutex<usize>>) {
    s.acquire();
    // perform work
    s.release();
    let mut d = done.lock().unwrap();
    *d += 1;
}

fn main() {
    let s = Arc::new(Semaphore::new(2));
    let done = Arc::new(Mutex::new(0));

    let mut handles = Vec::new();
    for _ in 0..3 {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        handles.push(thread::spawn(move || worker(s, done)));
    }

    for h in handles {
        h.join().unwrap();
    }

    let d = done.lock().unwrap();
    if *d == 3 {
        println!("DONE done=1");
    }
}
