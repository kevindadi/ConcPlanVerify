use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    ready: Mutex<bool>,
    condvar: Condvar,
}

fn main() {
    let shared = Arc::new(Shared {
        ready: Mutex::new(false),
        condvar: Condvar::new(),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let mut ready = waiter_shared.ready.lock().unwrap();
        while !*ready {
            ready = waiter_shared.condvar.wait(ready).unwrap();
        }
        println!("Waiter: ready is true, finishing.");
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let mut ready = notifier_shared.ready.lock().unwrap();
        *ready = true;
        notifier_shared.condvar.notify_one();
        println!("Notifier: set ready and signaled.");
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
