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
    // Two mutexes that workers A and B both need.
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));

    // Intermediate semaphore handshake between A and B.
    // A releases it after taking its first lock; B waits on it before
    // taking its second lock, and vice versa, so the two workers
    // interleave their acquisitions without deadlocking.
    let sem_a_to_b = Arc::new(Semaphore::new(0));
    let sem_b_to_a = Arc::new(Semaphore::new(0));

    // Bystander liveness flag: it keeps making progress forever.
    let bystander_running = Arc::new(Mutex::new(true));

    let mut handles = Vec::new();

    // Worker A: take m1, handshake, take m2, release handshake, finish.
    {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let sem_a_to_b = Arc::clone(&sem_a_to_b);
        let sem_b_to_a = Arc::clone(&sem_b_to_a);
        handles.push(thread::spawn(move || {
            // First lock.
            let _g1 = m1.lock().unwrap();
            // Signal B that A holds m1.
            sem_a_to_b.release();
            // Wait for B to hold m2 before A tries to take it.
            sem_b_to_a.acquire();
            // Second lock.
            let _g2 = m2.lock().unwrap();
            // Critical section complete; locks drop here.
        }));
    }

    // Worker B: take m2, handshake, take m1, release handshake, finish.
    {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let sem_a_to_b = Arc::clone(&sem_a_to_b);
        let sem_b_to_a = Arc::clone(&sem_b_to_a);
        handles.push(thread::spawn(move || {
            // First lock.
            let _g2 = m2.lock().unwrap();
            // Signal A that B holds m2.
            sem_b_to_a.release();
            // Wait for A to hold m1 before B tries to take it.
            sem_a_to_b.acquire();
            // Second lock.
            let _g1 = m1.lock().unwrap();
            // Critical section complete; locks drop here.
        }));
    }

    // Bystander: keeps making progress forever, independent of A and B.
    {
        let bystander_running = Arc::clone(&bystander_running);
        handles.push(thread::spawn(move || {
            let mut counter: u64 = 0;
            loop {
                counter = counter.wrapping_add(1);
                let running = bystander_running.lock().unwrap();
                if !*running {
                    break;
                }
                // Progress is made every iteration; the lock is released
                // immediately so A and B are never blocked by the bystander.
                drop(running);
            }
        }));
    }

    // Wait for A and B to complete.
    // The first two handles correspond to A and B.
    let a = handles.remove(0);
    let b = handles.remove(0);
    a.join().unwrap();
    b.join().unwrap();

    // Stop the bystander and join it.
    {
        let mut running = bystander_running.lock().unwrap();
        *running = false;
    }
    for h in handles {
        h.join().unwrap();
    }
}
