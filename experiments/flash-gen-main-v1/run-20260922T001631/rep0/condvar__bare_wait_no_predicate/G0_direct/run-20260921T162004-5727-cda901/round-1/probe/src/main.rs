use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    flag: bool,
}

fn main() {
    let shared = Arc::new((Mutex::new(Shared { flag: false }), Condvar::new()));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let (lock, cvar) = &*waiter_shared;
        let mut guard = lock.lock().unwrap();
        while !guard.flag {
            guard = cvar.wait(guard).unwrap();
        }
        // flag is true here
    });

    let notifier = thread::spawn(move || {
        let (lock, cvar) = &*notifier_shared;
        let mut guard = lock.lock().unwrap();
        guard.flag = true;
        cvar.notify_one();
        // lock released when guard drops
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _) = &*shared;
    let guard = lock.lock().unwrap();
    println!("DONE ready={}", guard.flag);
}
