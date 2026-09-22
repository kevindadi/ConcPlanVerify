use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicI64, Ordering};
use std::thread;

struct Semaphore {
    count: Mutex<i64>,
    cond: Condvar,
}

impl Semaphore {
    fn new(init: i64) -> Self {
        Semaphore {
            count: Mutex::new(init),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self, n: i64) {
        let mut c = self.count.lock().unwrap();
        while *c < n {
            c = self.cond.wait(c).unwrap();
        }
        *c -= n;
    }

    fn release(&self, n: i64) {
        let mut c = self.count.lock().unwrap();
        *c += n;
        self.cond.notify_all();
    }
}

static DONE: AtomicI64 = AtomicI64::new(0);

fn w2(s: Arc<Semaphore>) {
    s.acquire(1);
    s.release(1);
    s.acquire(1);
    s.release(1);
    let d = DONE.load(Ordering::SeqCst);
    let d2 = d + 1;
    DONE.store(d2, Ordering::SeqCst);
}

fn w1(s: Arc<Semaphore>) {
    let s1 = Arc::clone(&s);
    let h1 = thread::spawn(move || w2(s1));
    let s2 = Arc::clone(&s);
    let h2 = thread::spawn(move || w2(s2));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));
    let s1 = Arc::clone(&s);
    let sup = thread::spawn(move || w1(s1));
    sup.join().unwrap();
    println!("DONE done={}", DONE.load(Ordering::SeqCst));
}
