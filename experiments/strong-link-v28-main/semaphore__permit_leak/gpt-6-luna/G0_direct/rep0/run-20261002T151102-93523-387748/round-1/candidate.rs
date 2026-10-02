use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }
}

fn w1(s: Arc<Semaphore>) {
    s.acquire();
    let _work = (1u64..=1_000).fold(0u64, |sum, n| sum.wrapping_add(n));
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    let _work = (1u64..=1_000).fold(0u64, |sum, n| sum.wrapping_add(n));
    s.release();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let worker1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let worker2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE permits=1");
}
