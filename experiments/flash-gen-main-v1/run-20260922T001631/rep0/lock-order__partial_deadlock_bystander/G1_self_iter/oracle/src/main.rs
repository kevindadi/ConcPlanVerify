mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let lock_a = Arc::new(Lock::new());
    let lock_b = Arc::new(Lock::new());

    // Two permits used as a handshake.
    // permit1: worker1 signals it has taken its first lock.
    // permit2: worker2 signals it has taken its first lock.
    let permit1 = Arc::new(Semaphore::new(0));
    let permit2 = Arc::new(Semaphore::new(0));

    let done_a = Arc::new(Mutex::new_named("done_a_mutex0", false));
    let done_b = Arc::new(Mutex::new_named("done_b_mutex0", false));

    // Worker 1: takes lock_a first, then waits for worker2's signal, then lock_b.
    let (la1, lb1, p1a, p2a, da) = (
        Arc::clone(&lock_a),
        Arc::clone(&lock_b),
        Arc::clone(&permit1),
        Arc::clone(&permit2),
        Arc::clone(&done_a),
    );
    let w1 = cir_trace::spawn("w1", move || {
        la1.acquire();
        // Signal that we have taken our first lock.
        p1a.release();
        // Wait until worker2 has taken its first lock.
        p2a.acquire();
        lb1.acquire();
        // Critical section: hold both locks.
        // ... work ...
        lb1.release();
        la1.release();
        *da.lock().unwrap() = true;
    });

    // Worker 2: takes lock_b first, then waits for worker1's signal, then lock_a.
    let (la2, lb2, p1b, p2b, db) = (
        Arc::clone(&lock_a),
        Arc::clone(&lock_b),
        Arc::clone(&permit1),
        Arc::clone(&permit2),
        Arc::clone(&done_b),
    );
    let w2 = cir_trace::spawn("w2", move || {
        lb2.acquire();
        // Signal that we have taken our first lock.
        p2b.release();
        // Wait until worker1 has taken its first lock.
        p1b.acquire();
        la2.acquire();
        // Critical section: hold both locks.
        // ... work ...
        la2.release();
        lb2.release();
        *db.lock().unwrap() = true;
    });

    // Bystander: keeps making progress, never finishes on its own.
    let bystander = cir_trace::spawn("bystander", move || loop {
        thread::sleep(Duration::from_millis(10));
    });

    w1.join().unwrap();
    w2.join().unwrap();

    // Bystander keeps running; we don't join it.
    // Detach it by forgetting the handle.
    std::mem::forget(bystander);

    let a = *done_a.lock().unwrap();
    let b = *done_b.lock().unwrap();
    println!("DONE a={} b={}", a as u8, b as u8);
 cir_trace::finish();}
