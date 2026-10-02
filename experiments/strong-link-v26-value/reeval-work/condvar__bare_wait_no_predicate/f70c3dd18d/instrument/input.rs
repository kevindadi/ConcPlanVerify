use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(shared: &Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    // Lock the mutex guarding `ready` (R2).
    let mut ready = m.lock().unwrap();
    // Check the flag first; wait only while false; re-check after each wake (R4, R6, R9).
    while !*ready {
        // wait() releases the lock while blocked (R6) and reacquires it on wake (R4).
        ready = cv.wait(ready).unwrap();
    }
    drop(ready);
}

fn notifier(shared: &Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    // Set the flag to true while holding the lock (R3).
    let mut ready = m.lock().unwrap();
    *ready = true;
    // Signal while holding the lock, then release it (R3).
    cv.notify_all();
    drop(ready);
}

fn main() {
    // One lock and one condition variable guarding one boolean flag (R2).
    let shared = Arc::new((Mutex::new(false), Condvar::new()));
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    // Both roles run concurrently (R1).
    let w = thread::spawn(move || waiter(&s2));
    let n = thread::spawn(move || notifier(&s3));
    w.join().unwrap();
    n.join().unwrap();

    let ready = *shared.0.lock().unwrap();
    // Exactly one output line, then exit (R10).
    println!("DONE ready={}", ready);
}
