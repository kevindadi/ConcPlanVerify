use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A counting semaphore built from a mutex + condvar.
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            count: Mutex::new(initial),
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
    // taking its second lock, ensuring a well-defined interleaving.
    let handshake = Arc::new(Semaphore::new(0));

    // Bystander progress flag: it keeps making progress forever.
    let progress = Arc::new(Mutex::new(0u64));

    let m1_a = Arc::clone(&m1);
    let m2_a = Arc::clone(&m2);
    let hs_a = Arc::clone(&handshake);

    let worker_a = thread::spawn(move || {
        // Take first mutex.
        let _g1 = m1_a.lock().unwrap();
        // Signal B that A holds its first lock.
        hs_a.release();
        // Take second mutex.
        let _g2 = m2_a.lock().unwrap();
        // Critical section complete; both locks released on drop.
    });

    let m1_b = Arc::clone(&m1);
    let m2_b = Arc::clone(&m2);
    let hs_b = Arc::clone(&handshake);

    let worker_b = thread::spawn(move || {
        // Wait for A's handshake before taking the second mutex.
        hs_b.acquire();
        // Take first mutex.
        let _g1 = m1_b.lock().unwrap();
        // Take second mutex.
        let _g2 = m2_b.lock().unwrap();
        // Critical section complete; both locks released on drop.
    });

    let progress_bystander = Arc::clone(&progress);
    let bystander = thread::spawn(move || {
        // Independent task that keeps making progress forever.
        loop {
            let mut p = progress_bystander.lock().unwrap();
            *p = p.wrapping_add(1);
            // Release the lock promptly so workers are never starved.
            drop(p);
        }
    });

    // Join the two workers; both must complete.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // The bystander runs forever; we detach it by not joining.
    // To keep the program well-formed and terminating, we simply
    // let main exit, which ends the process and the bystander thread.
    drop(bystander);
}
