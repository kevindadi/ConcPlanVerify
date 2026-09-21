use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    let pair_w = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (mtx, cv) = &*pair_w;
        let mut ready = mtx.lock().unwrap();

        // Wait only while the predicate is false. If the notifier already
        // set `ready = true` before we got here, we never block and the
        // lost-wakeup race is avoided.
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        drop(ready);
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

    println!("DONE ready=true");
}
