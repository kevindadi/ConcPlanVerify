use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore { count: Mutex::new(count), cv: Condvar::new() }
    }
    fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 { c = self.cv.wait(c).unwrap(); }
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
    let permit1 = Arc::new(Semaphore::new(0));
    let permit2 = Arc::new(Semaphore::new(0));
    let result_a = Arc::new(Mutex::new(0));
    let result_b = Arc::new(Mutex::new(0));
    let stop = Arc::new((Mutex::new(false), Condvar::new()));

    // Worker A: first lock = lock_a
    let a_lock_a = Arc::clone(&lock_a);
    let a_lock_b = Arc::clone(&lock_b);
    let a_permit1 = Arc::clone(&permit1);
    let a_permit2 = Arc::clone(&permit2);
    let a_result = Arc::clone(&result_a);

    let worker_a = thread::spawn(move || {
        {
            let _g = a_lock_a.lock().unwrap();
            a_permit1.release();
            a_permit2.acquire();
        } // release first lock
        // Re-acquire in consistent global order: lock_a then lock_b
        let _g_a = a_lock_a.lock().unwrap();
        let _g_b = a_lock_b.lock().unwrap();
        *a_result.lock().unwrap() = 1;
    });

    // Worker B: first lock = lock_b
    let b_lock_a = Arc::clone(&lock_a);
    let b_lock_b = Arc::clone(&lock_b);
    let b_permit1 = Arc::clone(&permit1);
    let b_permit2 = Arc::clone(&permit2);
    let b_result = Arc::clone(&result_b);

    let worker_b = thread::spawn(move || {
        {
            let _g = b_lock_b.lock().unwrap();
            b_permit2.release();
            b_permit1.acquire();
        } // release first lock
        // Re-acquire in consistent global order: lock_a then lock_b
        let _g_a = b_lock_a.lock().unwrap();
        let _g_b = b_lock_b.lock().unwrap();
        *b_result.lock().unwrap() = 1;
    });

    let bystander_stop = Arc::clone(&stop);
    let bystander = thread::spawn(move || {
        let (m, cv) = &*bystander_stop;
        let mut done = m.lock().unwrap();
        while !*done {
            let (guard, _t) = cv.wait_timeout(done, std::time::Duration::from_millis(1)).unwrap();
            done = guard;
        }
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

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
