use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

// Two locks and two counting permits.
// We model each lock as a Mutex<bool> (held/not held) plus a Condvar to wait on.
// Permits are counting semaphores implemented with Mutex<usize> + Condvar.

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

    // Two permits used as a handshake.
    // permit1: worker1 signals it has taken its first lock.
    // permit2: worker2 signals it has taken its first lock.
    let permit1 = Arc::new(Semaphore::new(0));
    let permit2 = Arc::new(Semaphore::new(0));

    let done_a = Arc::new((Mutex::new(false), Condvar::new()));
    let done_b = Arc::new((Mutex::new(false), Condvar::new()));

    // Worker 1: takes lock_a first, then waits for worker2's signal, then lock_b.
    let (la, lb, p1, p2, da) = (
        lock_a.clone(),
        lock_b.clone(),
        permit1.clone(),
        permit2.clone(),
        done_a.clone(),
    );
    let w1 = thread::spawn(move || {
        la.acquire();
        // Signal that we have taken our first lock.
        p1.release();
        // Wait until worker2 has taken its first lock.
        p2.acquire();
        lb.acquire();
        // Critical section: hold both locks.
        // ... work ...
        lb.release();
        la.release();
        let (m, cv) = &*da;
        let mut d = m.lock().unwrap();
        *d = true;
        cv.notify_one();
    });

    // Worker 2: takes lock_b first, then waits for worker1's signal, then lock_a.
    let (la, lb, p1, p2, db) = (
        lock_a.clone(),
        lock_b.clone(),
        permit1.clone(),
        permit2.clone(),
        done_b.clone(),
    );
    let w2 = thread::spawn(move || {
        lb.acquire();
        // Signal that we have taken our first lock.
        p2.release();
        // Wait until worker1 has taken its first lock.
        p1.acquire();
        la.acquire();
        // Critical section: hold both locks.
        // ... work ...
        la.release();
        lb.release();
        let (m, cv) = &*db;
        let mut d = m.lock().unwrap();
        *d = true;
        cv.notify_one();
    });

    // Bystander: keeps making progress, never finishes on its own.
    let bystander = thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
    });

    // Main waits for both workers.
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

    // Bystander keeps running; we don't join it.
    let _ = bystander;

    println!("DONE a=1 b=1");
}
