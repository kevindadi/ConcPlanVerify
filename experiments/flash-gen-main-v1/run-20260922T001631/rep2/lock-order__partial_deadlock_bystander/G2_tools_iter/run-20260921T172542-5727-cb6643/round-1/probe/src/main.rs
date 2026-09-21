use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A simple counting semaphore built from Mutex + Condvar.
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
    // Two locks shared by the workers.
    let lock_a = Arc::new(Mutex::new(()));
    let lock_b = Arc::new(Mutex::new(()));

    // Two counting permits used as a handshake.
    // permit1: worker1 signals it has taken its first lock.
    // permit2: worker2 signals it has taken its first lock.
    let permit1 = Arc::new(Semaphore::new(0));
    let permit2 = Arc::new(Semaphore::new(0));

    // Results of the workers.
    let result = Arc::new(Mutex::new((0, 0)));

    // Bystander stop flag (never set; bystander keeps running).
    let bystander_running = Arc::new(Mutex::new(true));

    let mut handles = Vec::new();

    // Worker 1: takes lock_a first, then waits for worker2's signal,
    // then takes lock_b.
    {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let permit1 = Arc::clone(&permit1);
        let permit2 = Arc::clone(&permit2);
        let result = Arc::clone(&result);
        handles.push(thread::spawn(move || {
            // Take first lock.
            let _ga = lock_a.lock().unwrap();
            // Signal that we have taken our first lock.
            permit1.release();
            // Wait until the other worker has taken its first lock.
            permit2.acquire();
            // Take second lock.
            let _gb = lock_b.lock().unwrap();

            // Critical section: both locks held.
            {
                let mut r = result.lock().unwrap();
                r.0 = 1;
            }

            // Locks released automatically at end of scope.
        }));
    }

    // Worker 2: takes lock_b first, then waits for worker1's signal,
    // then takes lock_a.
    {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let permit1 = Arc::clone(&permit1);
        let permit2 = Arc::clone(&permit2);
        let result = Arc::clone(&result);
        handles.push(thread::spawn(move || {
            // Take first lock.
            let _gb = lock_b.lock().unwrap();
            // Signal that we have taken our first lock.
            permit2.release();
            // Wait until the other worker has taken its first lock.
            permit1.acquire();
            // Take second lock.
            let _ga = lock_a.lock().unwrap();

            // Critical section: both locks held.
            {
                let mut r = result.lock().unwrap();
                r.1 = 1;
            }

            // Locks released automatically at end of scope.
        }));
    }

    // Bystander: keeps making progress without ever finishing on its own.
    {
        let running = Arc::clone(&bystander_running);
        thread::spawn(move || loop {
            let r = running.lock().unwrap();
            if !*r {
                break;
            }
            drop(r);
            // Do a little work, then yield.
            std::hint::spin_loop();
            thread::yield_now();
        });
    }

    // Wait for both workers to finish.
    for h in handles {
        h.join().unwrap();
    }

    let r = result.lock().unwrap();
    println!("DONE a={} b={}", r.0, r.1);
}
