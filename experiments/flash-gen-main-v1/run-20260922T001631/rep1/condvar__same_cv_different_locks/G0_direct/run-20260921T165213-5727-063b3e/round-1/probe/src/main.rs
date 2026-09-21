use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    // Number of waiters that have announced they are about to wait.
    announced: Mutex<usize>,
    // Shared condition variable both waiters block on.
    cv: Condvar,
    // Locks each waiter holds while waiting.
    lock_a: Mutex<()>,
    lock_b: Mutex<()>,
}

fn main() {
    let shared = Arc::new(Shared {
        announced: Mutex::new(0),
        cv: Condvar::new(),
        lock_a: Mutex::new(()),
        lock_b: Mutex::new(()),
    });

    let s1 = Arc::clone(&shared);
    let waiter_a = thread::spawn(move || {
        // Waiter A holds lock_a while waiting on the shared condvar.
        let mut guard = s1.lock_a.lock().unwrap();

        // Announce before blocking.
        {
            let mut n = s1.announced.lock().unwrap();
            *n += 1;
            s1.cv.notify_all();
        }

        // Wait on the shared condition variable while holding lock_a.
        // The notifier will hold lock_a when it wakes us.
        while *s1.announced.lock().unwrap() < 2 {
            guard = s1.cv.wait(guard).unwrap();
        }

        // Woken and finished.
        drop(guard);
    });

    let s2 = Arc::clone(&shared);
    let waiter_b = thread::spawn(move || {
        // Waiter B holds lock_b while waiting on the shared condvar.
        let mut guard = s2.lock_b.lock().unwrap();

        // Announce before blocking.
        {
            let mut n = s2.announced.lock().unwrap();
            *n += 1;
            s2.cv.notify_all();
        }

        // Wait on the shared condition variable while holding lock_b.
        // The notifier will hold lock_b when it wakes us.
        while *s2.announced.lock().unwrap() < 2 {
            guard = s2.cv.wait(guard).unwrap();
        }

        // Woken and finished.
        drop(guard);
    });

    let s3 = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        // Wait until both waiters have announced themselves.
        {
            let mut n = s3.announced.lock().unwrap();
            while *n < 2 {
                n = s3.cv.wait(n).unwrap();
            }
        }

        // Wake the waiters while holding each lock they need to wake and finish.
        let _ga = s3.lock_a.lock().unwrap();
        let _gb = s3.lock_b.lock().unwrap();
        s3.cv.notify_all();
    });

    waiter_a.join().unwrap();
    waiter_b.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
}
