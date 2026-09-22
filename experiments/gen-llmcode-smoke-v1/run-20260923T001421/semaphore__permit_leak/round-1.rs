use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(init: usize) -> Self {
        Semaphore {
            count: Mutex::new(init),
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

fn w2(s: Arc<Semaphore>) {
    s.acquire(1);
    // work
    s.release(1);
}

fn w1(s: Arc<Semaphore>, done: Arc<Mutex<bool>>) {
    let h1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    h1.join().unwrap();
    h2.join().unwrap();
    *done.lock().unwrap() = true;
}

fn main() {
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(Mutex::new(false));

    let sup = thread::spawn({
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w1(s, done)
    });

    sup.join().unwrap();

    let permits = *s.count.lock().unwrap();
    println!("DONE permits={}", permits);
}
