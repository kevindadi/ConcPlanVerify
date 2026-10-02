use std::sync::{Arc, Mutex};
use std::thread;

// Sequential helper: only local computation, no synchronization.
fn compute() {
    let mut tmp = 0;
    tmp = 1;
    tmp = 2;
    let _ = tmp;
}

// Worker 1: lock m, compute, update shared acc, then unlock (guard drop).
fn w1(m: &Mutex<i64>) {
    let mut acc = m.lock().unwrap();
    compute();
    *acc = 1;
    // `acc` guard leaves scope here: mutex_unlock
}

// Worker 2: lock m, compute, update shared acc, then unlock (guard drop).
fn w2(m: &Mutex<i64>) {
    let mut acc = m.lock().unwrap();
    compute();
    *acc = 1;
    // `acc` guard leaves scope here: mutex_unlock
}

fn main() {
    // Shared resource m (Mutex) holds the shared variable acc, per the
    // protection edge main::acc -> main::m.
    let m = Arc::new(Mutex::new(0i64));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || {
        w1(&m1);
    });

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || {
        w2(&m2);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let value = *m.lock().unwrap();
    println!("DONE done={}", value);
}
