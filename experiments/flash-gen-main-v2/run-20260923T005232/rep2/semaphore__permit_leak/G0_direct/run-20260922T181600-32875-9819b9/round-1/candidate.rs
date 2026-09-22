use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
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

fn worker(s: Arc<Semaphore>) {
    s.acquire();
    // perform work while holding the permit
    s.release();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let w1 = thread::spawn(move || worker(s1));

    let s2 = Arc::clone(&s);
    let w2 = thread::spawn(move || worker(s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits={}", s.count());
}
