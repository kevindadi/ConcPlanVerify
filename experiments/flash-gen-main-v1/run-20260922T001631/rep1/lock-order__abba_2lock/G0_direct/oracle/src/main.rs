mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        lock_a: Mutex::new_named("shared_mutex0", false),
        lock_b: Mutex::new_named("shared_mutex1", false),
        cv_a: Condvar::new_named("shared_condvar0"),
        cv_b: Condvar::new_named("shared_condvar1"),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);

    let t1 = cir_trace::spawn("t1", move || worker(s1, 1));
    let t2 = cir_trace::spawn("t2", move || worker(s2, 2));

    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();

    println!("DONE t1={} t2={}", r1, r2);
 cir_trace::finish();}
