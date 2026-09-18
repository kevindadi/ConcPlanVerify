use std::sync::{Arc, Mutex, Condvar};
use std::thread;

// A simple counting semaphore implemented with Mutex + Condvar.
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

    fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cond.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cond.notify_one();
    }
}

fn main() {
    // Two mutexes shared by workers A and B.
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));

    // Semaphore handshake: A signals B after acquiring its first mutex.
    let handshake = Arc::new(Semaphore::new(0));

    // Bystander progress flag (never actually observed, just keeps it live).
    let progress = Arc::new(Mutex::new(0u64));

    let m1_a = Arc::clone(&m1);
    let m2_a = Arc::clone(&m2);
    let hs_a = Arc::clone(&handshake);

    let a = thread::spawn(move || {
        // A takes m1, then signals B, then takes m2.
        let _g1 = m1_a.lock().unwrap();
        hs_a.release();
        let _g2 = m2_a.lock().unwrap();
        // A completes.
    });

    let m1_b = Arc::clone(&m1);
    let m2_b = Arc::clone(&m2);
    let hs_b = Arc::clone(&handshake);

    let b = thread::spawn(move || {
        // B waits for A's handshake, then takes m1, then m2.
        // Using the same global lock order (m1 before m2) as A avoids
        // the circular wait that would otherwise deadlock the two workers.
        hs_b.acquire();
        let _g1 = m1_b.lock().unwrap();
        let _g2 = m2_b.lock().unwrap();
        // B completes.
    });

    let progress_b = Arc::clone(&progress);
    let bystander = thread::spawn(move || loop {
        let mut p = progress_b.lock().unwrap();
        *p = p.wrapping_add(1);
        // Release the lock each iteration so it never blocks others.
        drop(p);
        // Yield to allow other threads to run.
        thread::yield_now();
    });

    a.join().unwrap();
    b.join().unwrap();

    // Bystander runs forever; we don't join it.
    let _ = bystander;
}
