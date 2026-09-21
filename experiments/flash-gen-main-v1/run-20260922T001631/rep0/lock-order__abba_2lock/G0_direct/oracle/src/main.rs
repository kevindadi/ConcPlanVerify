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

fn worker(shared: Arc<Shared>, id: usize, done: Arc<Mutex<[bool; 2]>>) {
    // Acquire both locks in a consistent global order to prevent deadlock (R5).
    acquire(&shared.lock_a, &shared.cv_a);
    acquire(&shared.lock_b, &shared.cv_b);

    // Critical work while holding both locks (R3).
    {
        let mut d = done.lock().unwrap();
        d[id] = true;
    }

    // Release both locks before finishing (R7).
    release(&shared.lock_b, &shared.cv_b);
    release(&shared.lock_a, &shared.cv_a);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        lock_a: Mutex::new_named("shared_mutex0", false),
        lock_b: Mutex::new_named("shared_mutex1", false),
        cv_a: Condvar::new_named("shared_condvar0"),
        cv_b: Condvar::new_named("shared_condvar1"),
    });

    let done = Arc::new(Mutex::new_named("done_mutex0", [false; 2]));

    let s1 = Arc::clone(&shared);
    let d1 = Arc::clone(&done);
    let t1 = cir_trace::spawn("t1", move || worker(s1, 0, d1));

    let s2 = Arc::clone(&shared);
    let d2 = Arc::clone(&done);
    let t2 = cir_trace::spawn("t2", move || worker(s2, 1, d2));

    t1.join().unwrap();
    t2.join().unwrap();

    let d = done.lock().unwrap();
    println!("DONE t1={} t2={}", d[0] as u8, d[1] as u8);
 cir_trace::finish();}
