use std::sync::{Arc, Mutex, Condvar};
use std::thread;

// A simple counting semaphore implemented with Mutex + Condvar.
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
    // Two mutexes shared by workers A and B.
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));

    // Semaphore handshake: A signals B after it has finished its critical
    // section, so B never contends with A for the mutexes. This preserves
    // the handshake while eliminating the lock-order deadlock.
    let handshake = Arc::new(Semaphore::new(0));

    // Bystander progress flag (never blocks, just keeps making progress).
    let progress = Arc::new(Mutex::new(0u64));

    let m1_a = Arc::clone(&m1);
    let m2_a = Arc::clone(&m2);
    let hs_a = Arc::clone(&handshake);

    let a = thread::spawn(move || {
        // A takes m1 first.
        let g1 = m1_a.lock().unwrap();
        // Then A takes m2.
        let g2 = m2_a.lock().unwrap();
        // Critical section complete: release both locks before signaling.
        drop(g2);
        drop(g1);
        // Signal B that A is done with the shared mutexes.
        hs_a.release();
    });

    let m1_b = Arc::clone(&m1);
    let m2_b = Arc::clone(&m2);
    let hs_b = Arc::clone(&handshake);

    let b = thread::spawn(move || {
        // B waits for A's handshake before touching the mutexes, so the two
        // workers never hold conflicting locks at the same time.
        hs_b.acquire();
        // B takes m2 first.
        let _g2 = m2_b.lock().unwrap();
        // Then B takes m1.
        let _g1 = m1_b.lock().unwrap();
        // Critical section complete.
    });

    let progress_c = Arc::clone(&progress);
    let bystander = thread::spawn(move || {
        loop {
            let mut p = progress_c.lock().unwrap();
            *p = p.wrapping_add(1);
            // Release the lock promptly so it never blocks others.
            drop(p);
        }
    });

    a.join().unwrap();
    b.join().unwrap();

    // Bystander runs forever; we don't join it. Detach by dropping handle.
    drop(bystander);
}
