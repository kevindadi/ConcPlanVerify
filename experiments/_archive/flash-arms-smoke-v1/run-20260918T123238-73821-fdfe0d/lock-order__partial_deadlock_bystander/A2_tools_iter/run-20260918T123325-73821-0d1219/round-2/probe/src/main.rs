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

    // Bystander keeps making progress forever.
    let bystander_stop = Arc::new(Mutex::new(false));
    let bystander_stop_clone = Arc::clone(&bystander_stop);
    let bystander = thread::spawn(move || {
        let mut counter: u64 = 0;
        loop {
            counter = counter.wrapping_add(1);
            let stop = bystander_stop_clone.lock().unwrap();
            if *stop {
                break;
            }
            // Explicitly drop the lock before continuing.
            drop(stop);
            // Yield to allow other threads to run.
            thread::yield_now();
        }
    });

    // Worker A: acquire m1, signal handshake, acquire m2, release both.
    // Both workers acquire mutexes in the same global order (m1 then m2)
    // to prevent a circular-wait deadlock.
    let m1_a = Arc::clone(&m1);
    let m2_a = Arc::clone(&m2);
    let hs_a = Arc::clone(&handshake);
    let worker_a = thread::spawn(move || {
        let _g1 = m1_a.lock().unwrap();
        hs_a.release();
        let _g2 = m2_a.lock().unwrap();
        // Both mutexes held; work done.
    });

    // Worker B: wait for handshake, acquire m1, acquire m2, release both.
    // Uses the same lock order as A (m1 then m2) to avoid deadlock.
    let m1_b = Arc::clone(&m1);
    let m2_b = Arc::clone(&m2);
    let hs_b = Arc::clone(&handshake);
    let worker_b = thread::spawn(move || {
        hs_b.acquire();
        let _g1 = m1_b.lock().unwrap();
        let _g2 = m2_b.lock().unwrap();
        // Both mutexes held; work done.
    });

    // Wait for both workers to complete.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Stop the bystander.
    {
        let mut stop = bystander_stop.lock().unwrap();
        *stop = true;
    }
    bystander.join().unwrap();
}
