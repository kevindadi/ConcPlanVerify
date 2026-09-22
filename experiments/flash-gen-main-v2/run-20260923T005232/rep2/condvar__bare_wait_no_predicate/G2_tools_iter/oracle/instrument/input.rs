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
        // flag is true here
    });

    let notifier = thread::spawn(move || {
        let (lock, cv) = &*m;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cv) = &*Arc::new((Mutex::new(false), Condvar::new()));
    let _ = lock;
    println!("DONE ready=true");
}
