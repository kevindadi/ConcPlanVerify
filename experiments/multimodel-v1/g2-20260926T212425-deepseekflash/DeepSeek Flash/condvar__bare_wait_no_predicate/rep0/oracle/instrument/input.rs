use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new((Mutex::new(false), Condvar::new()));
    let m2 = Arc::clone(&m);

    let waiter = thread::spawn(move || {
        let (lock, cv) = &*m2;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        *ready
    });

    let notifier = thread::spawn(move || {
        let (lock, cv) = &*m;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    let ready = waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready={}", ready);
}
