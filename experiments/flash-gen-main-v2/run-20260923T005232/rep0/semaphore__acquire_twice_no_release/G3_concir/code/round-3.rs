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

    fn acquire(&self, n: usize) {
        let mut c = self.count.lock().unwrap();
        while *c < n {
            c = self.cond.wait(c).unwrap();
        }
        *c -= n;
    }

    fn release(&self, n: usize) {
        let mut c = self.count.lock().unwrap();
        *c += n;
        self.cond.notify_all();
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let w1 = thread::spawn(move || {
        s1.acquire(1);
        s1.release(1);
    });

    let s2 = Arc::clone(&s);
    let w2 = thread::spawn(move || {
        s2.acquire(1);
        s2.release(1);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
