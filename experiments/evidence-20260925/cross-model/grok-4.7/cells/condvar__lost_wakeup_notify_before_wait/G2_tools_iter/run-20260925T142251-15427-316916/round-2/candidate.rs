use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use concir_sync::Semaphore;

fn main() {
    let sem = Semaphore::new(1);
    let permit = sem.acquire();
    drop(permit);

    let state = Arc::new((Mutex::new(false), Condvar::new()));

    let state_waiter = Arc::clone(&state);
    let waiter = thread::spawn(move || {
        let (m, cv) = &*state_waiter;
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
    });

    let state_notifier = Arc::clone(&state);
    let notifier = thread::spawn(move || {
        let (m, cv) = &*state_notifier;
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (m, _cv) = &*state;
    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
