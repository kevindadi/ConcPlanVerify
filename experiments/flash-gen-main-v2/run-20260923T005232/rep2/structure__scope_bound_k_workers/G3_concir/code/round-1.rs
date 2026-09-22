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
        let mut count = self.count.lock().unwrap();
        while *count < n {
            count = self.cond.wait(count).unwrap();
        }
        *count -= n;
    }

    fn release(&self, n: usize) {
        let mut count = self.count.lock().unwrap();
        *count += n;
        self.cond.notify_all();
    }
}

fn w1(s: Arc<Semaphore>) {
    s.acquire(1);
    s.release(1);
}

fn w2(s: Arc<Semaphore>) {
    s.acquire(1);
    s.release(1);
}

fn w3(s: Arc<Semaphore>) {
    s.acquire(1);
    s.release(1);
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));
    let h3 = thread::spawn(move || w3(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
