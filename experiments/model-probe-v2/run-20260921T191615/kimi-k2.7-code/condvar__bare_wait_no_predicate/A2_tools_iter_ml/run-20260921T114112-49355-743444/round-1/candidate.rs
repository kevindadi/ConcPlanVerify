use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    let pair_w = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (mtx, cv) = &*pair_w;
        let guard = mtx.lock().unwrap();
        let _guard = cv.wait_while(guard, |ready| !*ready).unwrap();
    });

    let pair_n = Arc::clone(&pair);
    let notifier = thread::spawn(move || {
        let (mtx, cv) = &*pair_n;
        let mut ready = mtx.lock().unwrap();
        *ready = true;
        cv.notify_all();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = *pair.0.lock().unwrap();
    println!("DONE ready={}", ready);
}
