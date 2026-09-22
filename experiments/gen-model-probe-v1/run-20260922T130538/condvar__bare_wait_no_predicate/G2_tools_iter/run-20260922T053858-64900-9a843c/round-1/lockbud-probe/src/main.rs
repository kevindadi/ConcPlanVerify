use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let waiter_pair = Arc::clone(&pair);
    let notifier_pair = Arc::clone(&pair);

    let waiter = thread::spawn(move || {
        let (lock, cvar) = &*waiter_pair;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
    });

    let notifier = thread::spawn(move || {
        let (lock, cvar) = &*notifier_pair;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _) = &*pair;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
}
