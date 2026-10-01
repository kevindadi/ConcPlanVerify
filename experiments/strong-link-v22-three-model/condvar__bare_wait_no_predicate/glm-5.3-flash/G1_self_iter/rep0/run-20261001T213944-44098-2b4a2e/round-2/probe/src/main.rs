use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(shared: &Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(shared: &Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_all();
    drop(ready);
}

fn main() {
    let shared = Arc::new((Mutex::new(false), Condvar::new()));
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let w = thread::spawn(move || waiter(&s2));
    let n = thread::spawn(move || notifier(&s3));
    w.join().unwrap();
    n.join().unwrap();

    let ready = *shared.0.lock().unwrap();
    println!("DONE ready={}", ready);
}
