use std::sync::{Arc, Condvar, Mutex};
use std::thread;

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
    let lock_a = Arc::new(Mutex::new(()));
    let lock_b = Arc::new(Mutex::new(()));

    // permit1: worker1 signals it has taken its first lock.
    // permit2: worker2 signals it has taken its first lock.
    let permit1 = Arc::new(Semaphore::new(0));
    let permit2 = Arc::new(Semaphore::new(0));

    let result = Arc::new(Mutex::new((0, 0)));

    let bystander_running = Arc::new(Mutex::new(true));

    let mut handles = Vec::new();

    // Worker 1: takes lock_a first, signals permit1, waits for permit2,
    // then takes lock_b. Both workers acquire locks in the same order
    // (a then b) to avoid AB-BA deadlock.
    {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let permit1 = Arc::clone(&permit1);
        let permit2 = Arc::clone(&permit2);
        let result = Arc::clone(&result);
        handles.push(thread::spawn(move || {
            let _ga = lock_a.lock().unwrap();
            permit1.release();
            permit2.acquire();
            let _gb = lock_b.lock().unwrap();

            {
                let mut r = result.lock().unwrap();
                r.0 = 1;
            }
        }));
    }

    // Worker 2: waits for permit1 (worker1 has taken lock_a), then takes
    // lock_a, signals permit2, then takes lock_b. Same lock order as worker1
