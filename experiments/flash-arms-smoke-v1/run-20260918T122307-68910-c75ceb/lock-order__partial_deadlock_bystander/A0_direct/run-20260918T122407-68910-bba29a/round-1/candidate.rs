use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A counting semaphore built from a mutex + condvar.
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
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

fn main() {
    // Two mutexes that workers A and B contend on.
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));

    // Intermediate semaphore handshake between A and B.
    // A releases it after taking its first lock; B waits on it before
    // taking its second lock. This orders the lock acquisition so that
    // no deadlock cycle can form.
    let handshake = Arc::new(Semaphore::new(0));

    // Bystander progress flag: it keeps making progress forever.
    let progress = Arc::new(Mutex::new(0u64));

    let a_m1 = Arc::clone(&m1);
    let a_m2 = Arc::clone(&m2);
    let a_hs = Arc::clone(&handshake);

    let b_m1 = Arc::clone(&m1);
    let b_m2 = Arc::clone(&m2);
    let b_hs = Arc::clone(&handshake);

    let p = Arc::clone(&progress);

    // Worker A: takes m1, signals handshake, then takes m2.
    let worker_a = thread::spawn(move || {
        let _g1 = a_m1.lock().unwrap();
        // Signal that A holds its first lock.
        a_hs.release();
        let _g2 = a_m2.lock().unwrap();
        // Critical section complete; both locks held and released here.
    });

    // Worker B: waits for A's handshake, then takes m2, then m1.
    let worker_b = thread::spawn(move || {
        // Wait until A has taken its first lock before proceeding.
        b_hs.acquire();
        let _g2 = b_m2.lock().unwrap();
        let _g1 = b_m1.lock().unwrap();
        // Critical section complete.
    });

    // Bystander: keeps making progress forever, independent of A and B.
    let bystander = thread::spawn(move || {
        loop {
            let mut v = p.lock().unwrap();
            *v = v.wrapping_add(1);
            // Release the lock each iteration so it is not a permanent
            // blocker; the bystander remains globally live.
            drop(v);
        }
    });

    // Join the two workers; both must complete.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // The bystander runs forever, so we do not join it. To keep the
    // program well-formed and terminating, we detach it by dropping the
    // handle. Main terminates after A and B complete.
    drop(bystander);
}
