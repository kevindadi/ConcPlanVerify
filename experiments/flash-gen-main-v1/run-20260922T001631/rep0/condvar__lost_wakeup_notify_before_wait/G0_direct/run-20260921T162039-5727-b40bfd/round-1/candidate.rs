use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    flag: bool,
    lock: Mutex<()>,
    cond: Condvar,
}

fn main() {
    let shared = Arc::new(Shared {
        flag: false,
        lock: Mutex::new(()),
        cond: Condvar::new(),
    });

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let mut guard = waiter_shared.lock.lock().unwrap();
        while !waiter_shared.flag {
            guard = waiter_shared.cond.wait(guard).unwrap();
        }
    });

    let notifier = thread::spawn(move || {
        let _guard = notifier_shared.lock.lock().unwrap();
        notifier_shared.flag = true;
        notifier_shared.cond.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = {
        let _guard = shared.lock.lock().unwrap();
        shared.flag
    };

    println!("DONE ready={}", ready);
}
