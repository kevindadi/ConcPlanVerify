use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let flag = Arc::new((Mutex::new(false), Condvar::new()));

    let waiter_flag = Arc::clone(&flag);
    let waiter = thread::spawn(move || {
        let (lock, cvar) = &*waiter_flag;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
    });

    let notifier_flag = Arc::clone(&flag);
    let notifier = thread::spawn(move || {
        let (lock, cvar) = &*notifier_flag;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _) = &*flag;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
}
