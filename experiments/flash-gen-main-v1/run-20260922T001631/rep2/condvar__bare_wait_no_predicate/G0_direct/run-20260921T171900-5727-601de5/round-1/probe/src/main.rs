use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    flag: Mutex<bool>,
    cond: Condvar,
}

fn main() {
    let shared = Arc::new(Shared {
        flag: Mutex::new(false),
        cond: Condvar::new(),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let mut flag = waiter_shared.flag.lock().unwrap();
        while !*flag {
            flag = waiter_shared.cond.wait(flag).unwrap();
        }
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let mut flag = notifier_shared.flag.lock().unwrap();
        *flag = true;
        notifier_shared.cond.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = *shared.flag.lock().unwrap();
    println!("DONE ready={}", ready);
}
