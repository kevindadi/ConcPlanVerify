use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter = thread::spawn(move || {
        let mut ready = waiter_m.lock().unwrap();
        while !*ready {
            ready = waiter_cv.wait(ready).unwrap();
        }
    });

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier = thread::spawn(move || {
        let mut ready = notifier_m.lock().unwrap();
        *ready = true;
        notifier_cv.notify_one();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready=true");
}
