use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared state: a flag protected by the mutex, plus the condition variable.
    let p = Arc::new((Mutex::new(false), Condvar::new()));
    let mut hs = vec![];

    // Spawn the waiters. Each waits until the flag is set.
    for _ in 0..2 {
        let q = Arc::clone(&p);
        hs.push(thread::spawn(move || {
            let (m, cv) = &*q;
            let mut g = m.lock().unwrap();
            while !*g {
                g = cv.wait(g).unwrap();
            }
            // Propagate the wakeup so every waiter is guaranteed to observe
            // the flag and complete, regardless of interleaving.
            cv.notify_one();
        }));
    }

    // The notifier sets the flag and wakes a waiter.
    let q = Arc::clone(&p);
    let n = thread::spawn(move || {
        let (m, cv) = &*q;
        let mut g = m.lock().unwrap();
        *g = true;
        cv.notify_one();
    });

    n.join().unwrap();
    for h in hs {
        h.join().unwrap();
    }
    println!("DONE waiters=0");
}
