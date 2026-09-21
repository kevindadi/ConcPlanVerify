use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    let pair_w = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (mtx, cv) = &*pair_w;
        let mut guard = mtx.lock().unwrap();
        while !*guard {
            guard = cv.wait(guard).unwrap();
        }
        drop(guard);
        println!("DONE ready=true");
    });

    let pair_n = Arc::clone(&pair);
    let notifier = thread::spawn(move || {
        let (mtx, cv) = &*pair_n;
        let mut ready = mtx.lock().unwrap();
        *ready = true;
        cv.notify_all();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
