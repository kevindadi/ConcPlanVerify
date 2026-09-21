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

    // Bystander stop flag.
    let stop = Arc::new(Mutex::new(false));
    let stop_cv = Arc::new(Condvar::new());

    // Worker 1: takes lock_a first, then waits for worker2's signal,
    // then takes lock_b.
    let w1_lock_a = Arc::clone(&lock_a);
    let w1_lock_b = Arc::clone(&lock_b);
    let w1_permit1 = Arc::clone(&permit1);
    let w1_permit2 = Arc::clone(&permit2);
    let w1_result = Arc::clone(&result);

    let worker1 = thread::spawn(move || {
        // Take first lock.
        let _g_a = w1_lock_a.lock().unwrap();

        // Signal that we have taken our first lock.
        w1_permit1.release();

        // Wait until the other worker has taken its first lock.
        w1_permit2.acquire();

        // Take second lock while still holding the first.
        let _g_b = w1_lock_b.lock().unwrap();

        // Critical section: both locks held.
        {
            let mut r = w1_result.lock().unwrap();
            r.0 = 1;
        }

        // Locks released automatically when guards drop.
    });

    // Worker 2: takes lock_b first, then waits for worker1's signal,
    // then takes lock_a.
    let w2_lock_a = Arc::clone(&lock_a);
    let w2_lock_b = Arc::clone(&lock_b);
    let w2_permit1 = Arc::clone(&permit1);
    let w2_permit2 = Arc::clone(&permit2);
    let w2_result = Arc::clone(&result);

    let worker2 = thread::spawn(move || {
        // Take first lock.
        let _g_b = w2_lock_b.lock().unwrap();

        // Signal that we have taken our first lock.
        w2_permit2.release();

        // Wait until the other worker has taken its first lock.
        w2_permit1.acquire();

        // Take second lock while still holding the first.
        let _g_a = w2_lock_a.lock().unwrap();

        // Critical section: both locks held.
        {
            let mut r = w2_result.lock().unwrap();
            r.1 = 1;
        }

        // Locks released automatically when guards drop.
    });

    // Bystander: keeps making progress without ever finishing on its own.
    let b_stop = Arc::clone(&stop);
    let b_cv = Arc::clone(&stop_cv);

    let bystander = thread::spawn(move || {
        loop {
            let mut s = b_stop.lock().unwrap();
            if *s {
                break;
            }
            // Do a bit of work, then check again.
            // Use a short wait so we don't spin too hard, but still
            // make progress and remain responsive to the stop flag.
            let (guard, _timeout) = b_cv
                .wait_timeout(s, std::time::Duration::from_millis(1))
                .unwrap();
            s = guard;
            if *s {
                break;
            }
        }
    });

    // Main thread waits for both workers to finish.
    worker1.join().unwrap();
    worker2.join().unwrap();

    // Signal the bystander to stop and wait for it.
    {
        let mut s = stop.lock().unwrap();
        *s = true;
        stop_cv.notify_all();
    }
    bystander.join().unwrap();

    let r = result.lock().unwrap();
    println!("DONE a={} b={}", r.0, r.1);
}
