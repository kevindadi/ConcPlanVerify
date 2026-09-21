use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Semaphore {
            count: Mutex::new(n),
            cv: Condvar::new(),
        }
    }
    fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }
    fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cv.notify_one();
    }
}

struct Lock {
    held: Mutex<bool>,
    cv: Condvar,
}

impl Lock {
    fn new() -> Self {
        Lock {
            held: Mutex::new(false),
            cv: Condvar::new(),
        }
    }
    fn acquire(&self) {
        let mut h = self.held.lock().unwrap();
        while *h {
            h = self.cv.wait(h).unwrap();
        }
        *h = true;
    }
    fn release(&self) {
        let mut h = self.held.lock().unwrap();
        *h = false;
        self.cv.notify_one();
    }
}

fn main() {
    let lock_a = Arc::new(Lock::new());
    let lock_b = Arc::new(Lock::new());

    let permit1 = Arc::new(Semaphore::new(0));
    let permit2 = Arc::new(Semaphore::new(0));

    let done_a = Arc::new((Mutex::new(false), Condvar::new()));
    let done_b = Arc::new((Mutex::new(false), Condvar::new()));

    // Worker 1: handshake first, then acquire both locks in consistent order.
    let (la, lb, p1, p2, da) = (
        lock_a.clone(),
        lock_b.clone(),
        permit1.clone(),
        permit2.clone(),
        done_a.clone(),
    );
    let w1 = thread::spawn(move || {
        // Handshake: signal readiness, wait for the other worker.
        p1.release();
        p2.acquire();
        // Now acquire both locks in consistent global order.
        la.acquire();
        lb.acquire();
        // Critical section: hold both locks.
        lb.release();
        la.release();
        let (m, cv) = &*da;
        let mut d = m.lock().unwrap();
        *d = true;
        cv.notify_one();
    });

    // Worker 2: handshake first, then acquire both locks in consistent order.
    let (la, lb, p1, p2, db) = (
        lock_a.clone(),
        lock_b.clone(),
        permit1.clone(),
        permit2.clone(),
        done_b.clone(),
    );
    let w2 = thread::spawn(move || {
        // Handshake: signal readiness, wait for the other worker.
        p2.release();
        p1.acquire();
        // Now acquire both locks in consistent global order.
        la.acquire();
        lb.acquire();
        // Critical section: hold both locks.
        lb.release();
        la.release();
        let (m, cv) = &*db;
        let mut d = m.lock().unwrap();
        *d = true;
        cv.notify_one();
    });

    let bystander = thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
    });

    {
        let (m, cv) = &*done_a;
        let mut d = m.lock().unwrap();
        while !*d {
            d = cv.wait(d).unwrap();
        }
    }
    {
        let (m, cv) = &*done_b;
        let mut d = m.lock().unwrap();
        while !*d {
            d = cv.wait(d).unwrap();
        }
    }

    w1.join().unwrap();
    w2.join().unwrap();

    let _ = bystander;

    println!("DONE a=1 b=1");
}
