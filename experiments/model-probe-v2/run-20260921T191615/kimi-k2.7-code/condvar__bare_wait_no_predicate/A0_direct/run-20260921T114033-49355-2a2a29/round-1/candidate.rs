use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    let pair_w = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (mtx, cv) = &*pair_w;
        let guard = mtx.lock().unwrap();

        // Re-check the predicate in a loop so the waiter completes
        // even if the notification arrives before the wait begins.
        let guard = cv.wait_while(guard, |ready| !*ready).unwrap();
        let ready = *guard;
        drop(guard);

        println!("DONE ready={}", ready);
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
