use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            count: Mutex::new(permits),
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

fn worker(s: Arc<Semaphore>, id: usize) {
    // Each worker acquires the permit multiple times, doing work while holding it,
    // and releases exactly as many times as it acquired.
    let acquisitions = if id == 1 { 3 } else { 2 };

    for _ in 0..acquisitions {
        s.acquire();
        // Critical section: perform work while holding the permit.
        // (No other worker can hold the permit simultaneously.)
        s.release();
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = thread::spawn(move || worker(s1, 1));
    let w2 = thread::spawn(move || worker(s2, 2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
