use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    let waiter = thread::spawn(move || {
        let (lock, cvar) = &*pair2;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
    });

    let notifier = thread::spawn(move || {
        let (lock, cvar) = &*pair;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
