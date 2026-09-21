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
    // permit1: worker A signals it has taken its first lock.
    // permit2: worker B signals it has taken its first lock.
    let permit1 = Arc::new(Semaphore::new(0));
    let permit2 = Arc::new(Semaphore::new(0));

    // Results of the workers.
    let result_a = Arc::new(Mutex::new(0));
    let result_b = Arc::new(Mutex::new(0));

    // Bystander stop flag.
    let stop = Arc::new((Mutex::new(false), Condvar::new()));

    // Worker A: takes lock_a first, then waits for B's first lock, then lock_b.
    let a_lock_a = Arc::clone(&lock_a);
    let a_lock_b = Arc::clone(&lock_b);
    let a_permit1 = Arc::clone(&permit1);
    let a_permit2 = Arc::clone(&permit2);
    let a_result = Arc::clone(&result_a);

    let worker_a = thread::spawn(move || {
        // Take first lock.
        let _g_a = a_lock_a.lock().unwrap();

        // Signal that we have taken our first lock.
        a_permit1.release();

        // Wait until the other worker has taken its first lock.
        a_permit2.acquire();

        // Now take the second lock.
        let _g_b = a_lock_b.lock().unwrap();

        // Critical section: both locks held.
        *a_result.lock().unwrap() = 1;

        // Locks released automatically when guards drop.
    });

    // Worker B: takes lock_b first, then waits for A's first lock, then lock_a.
    let b_lock_a = Arc::clone(&lock_a);
    let b_lock_b = Arc::clone(&lock_b);
    let b_permit1 = Arc::clone(&permit1);
    let b_permit2 = Arc::clone(&permit2);
    let b_result = Arc::clone(&result_b);

    let worker_b = thread::spawn(move || {
        // Take first lock.
        let _g_b = b_lock_b.lock().unwrap();

        // Signal that we have taken our first lock.
        b_permit2.release();

        // Wait until the other worker has taken its first lock.
        b_permit1.acquire();

        // Now take the second lock.
        let _g_a = b_lock_a.lock().unwrap();

        // Critical section: both locks held.
        *b_result.lock().unwrap() = 1;

        // Locks released automatically when guards drop.
    });

    // Bystander: keeps making progress without ever finishing on its own.
    let bystander_stop = Arc::clone(&stop);
    let bystander = thread::spawn(move || {
        let (m, cv) = &*bystander_stop;
        let mut done = m.lock().unwrap();
        while !*done {
            // Do a bit of work, then check whether we should stop.
            let _ = std::hint::spin_loop();
            let (guard, _timeout) = cv
                .wait_timeout(done, std::time::Duration::from_millis(1))
                .unwrap();
            done = guard;
        }
    });

    // Main thread waits for both workers to finish.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Tell the bystander to stop, then join it.
    {
        let (m, cv) = &*stop;
        let mut done = m.lock().unwrap();
        *done = true;
        cv.notify_all();
    }
    bystander.join().unwrap();

    let a = *result_a.lock().unwrap();
    let b = *result_b.lock().unwrap();
    println!("DONE a={} b={}", a, b);
}
