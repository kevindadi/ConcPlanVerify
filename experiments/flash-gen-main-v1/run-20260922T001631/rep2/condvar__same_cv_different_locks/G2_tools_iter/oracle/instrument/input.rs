use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    // Number of waiters that have announced they are about to wait.
    announced: Mutex<usize>,
    // Condition variable shared by both waiters.
    cv: Condvar,
}

fn main() {
    let shared = Arc::new(Shared {
        announced: Mutex::new(0),
        cv: Condvar::new(),
    });

    // Two different locks, one per waiter.
    let lock1 = Arc::new(Mutex::new(()));
    let lock2 = Arc::new(Mutex::new(()));

    let s1 = Arc::clone(&shared);
    let l1 = Arc::clone(&lock1);
    let waiter1 = thread::spawn(move || {
        let mut guard = l1.lock().unwrap();
        // Announce before waiting.
        {
            let mut n = s1.announced.lock().unwrap();
            *n += 1;
            s1.cv.notify_all();
        }
        // Wait while holding our own lock.
        while *s1.announced.lock().unwrap() < 2 {
            guard = s1.cv.wait(guard).unwrap();
        }
        // Woken; finish while still holding the lock.
        drop(guard);
    });

    let s2 = Arc::clone(&shared);
    let l2 = Arc::clone(&lock2);
    let waiter2 = thread::spawn(move || {
        let mut guard = l2.lock().unwrap();
        {
            let mut n = s2.announced.lock().unwrap();
            *n += 1;
            s2.cv.notify_all();
        }
        while *s2.announced.lock().unwrap() < 2 {
            guard = s2.cv.wait(guard).unwrap();
        }
        drop(guard);
    });

    let s3 = Arc::clone(&shared);
    let l1n = Arc::clone(&lock1);
    let l2n = Arc::clone(&lock2);
    let notifier = thread::spawn(move || {
        // Wait until both waiters have announced.
        {
            let mut n = s3.announced.lock().unwrap();
            while *n < 2 {
                n = s3.cv.wait(n).unwrap();
            }
        }
        // Hold each lock that a waiter needs in order to wake and finish.
        let _g1 = l1n.lock().unwrap();
        let _g2 = l2n.lock().unwrap();
        // Wake both waiters.
        s3.cv.notify_all();
    });

    waiter1.join().unwrap();
    waiter2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
}
