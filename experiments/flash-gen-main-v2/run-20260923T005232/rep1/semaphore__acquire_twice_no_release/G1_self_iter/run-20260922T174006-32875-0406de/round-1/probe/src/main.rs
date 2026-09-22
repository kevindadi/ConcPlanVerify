use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A simple counting semaphore built from a Mutex + Condvar.
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

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let w1 = thread::spawn(move || {
        s1.acquire();
        // work
        s1.release();
    });

    let s2 = Arc::clone(&s);
    let w2 = thread::spawn(move || {
        s2.acquire();
        // work
        s2.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
