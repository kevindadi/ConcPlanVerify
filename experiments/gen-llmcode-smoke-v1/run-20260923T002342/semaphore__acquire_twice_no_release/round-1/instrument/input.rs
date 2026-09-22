use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
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

fn w2(s: Arc<Semaphore>, done: Arc<AtomicI64>) {
    s.acquire(1);
    s.release(1);
    s.acquire(1);
    s.release(1);
    let d = done.load(Ordering::SeqCst);
    let d2 = d + 1;
    done.store(d2, Ordering::SeqCst);
}

fn w1(s: Arc<Semaphore>, done: Arc<AtomicI64>) {
    let s1 = Arc::clone(&s);
    let d1 = Arc::clone(&done);
    let w1_handle = thread::spawn(move || w2(s1, d1));

    let s2 = Arc::clone(&s);
    let d2 = Arc::clone(&done);
    let w2_handle = thread::spawn(move || w2(s2, d2));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));
    let done = Arc::new(AtomicI64::new(0));

    let s1 = Arc::clone(&s);
    let d1 = Arc::clone(&done);
    let sup = thread::spawn(move || w1(s1, d1));

    sup.join().unwrap();

    let d = done.load(Ordering::SeqCst);
    println!("DONE done={}", d);
}
