use std::sync::{Arc, Mutex, Condvar};
use std::thread;

struct Shared {
    lock_a: Mutex<bool>,
    lock_b: Mutex<bool>,
    cv_a: Condvar,
    cv_b: Condvar,
}

fn acquire(lock: &Mutex<bool>, cv: &Condvar) {
    let mut held = lock.lock().unwrap();
    while *held {
        held = cv.wait(held).unwrap();
    }
    *held = true;
}

fn release(lock: &Mutex<bool>, cv: &Condvar) {
    let mut held = lock.lock().unwrap();
    *held = false;
    cv.notify_one();
}

fn worker(shared: Arc<Shared>, id: usize) -> usize {
    // Acquire both locks in a consistent global order to prevent deadlock (R5).
    acquire(&shared.lock_a, &shared.cv_a);
    acquire(&shared.lock_b, &shared.cv_b);

    // Critical work while holding both locks (R3).
    let _ = id;

    // Release both locks before finishing (R7).
    release(&shared.lock_b, &shared.cv_b);
    release(&shared.lock_a, &shared.cv_a);

    1
}

fn main() {
    let shared = Arc::new(Shared {
        lock_a: Mutex::new(false),
        lock_b: Mutex::new(false),
        cv_a: Condvar::new(),
        cv_b: Condvar::new(),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);

    let t1 = thread::spawn(move || worker(s1, 1));
    let t2 = thread::spawn(move || worker(s2, 2));

    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();

    println!("DONE t1={} t2={}", r1, r2);
}
