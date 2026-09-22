use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new((Mutex::new(false), Condvar::new()));

    let waiter_m = Arc::clone(&m);
    let waiter = thread::spawn(move || {
        let (lock, cv) = &*waiter_m;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        // flag is true here
    });

    let notifier_m = Arc::clone(&m);
    let notifier = thread::spawn(move || {
        let (lock, cv) = &*notifier_m;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cv) = &*m;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
}
