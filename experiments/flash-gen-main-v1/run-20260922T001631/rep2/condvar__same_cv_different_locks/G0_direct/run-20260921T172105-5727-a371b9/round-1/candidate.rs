use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    // Number of waiters that have announced they are about to wait.
    announced: Mutex<usize>,
    // Shared condition variable both waiters block on.
    cv: Condvar,
}

fn main() {
    let shared = Arc::new(Shared {
        announced: Mutex::new(0),
        cv: Condvar::new(),
    });

    // Two different locks, one per waiter.
    let lock_a = Arc::new(Mutex::new(()));
    let lock_b = Arc::new(Mutex::new(()));

    let shared_w1 = Arc::clone(&shared);
    let lock_a_w1 = Arc::clone(&lock_a);

    let shared_w2 = Arc::clone(&shared);
    let lock_b_w2 = Arc::clone(&lock_b);

    let shared_n = Arc::clone(&shared);
    let lock_a_n = Arc::clone(&lock_a);
    let lock_b_n = Arc::clone(&lock_b);

    // Waiter 1: holds lock_a while waiting on the shared condvar.
    let w1 = thread::spawn(move || {
        let mut guard = lock_a_w1.lock().unwrap();

        // Announce that we are about to wait.
        {
            let mut count = shared_w1.announced.lock().unwrap();
            *count += 1;
            shared_w1.cv.notify_all();
        }

        // Wait on the shared condvar while holding lock_a.
        while *shared_w1.announced.lock().unwrap() < 3 {
            guard = shared_w1.cv.wait(guard).unwrap();
        }

        // Woken and finished.
        drop(guard);
    });

    // Waiter 2: holds lock_b while waiting on the shared condvar.
    let w2 = thread::spawn(move || {
        let mut guard = lock_b_w2.lock().unwrap();

        // Announce that we are about to wait.
        {
            let mut count = shared_w2.announced.lock().unwrap();
            *count += 1;
            shared_w2.cv.notify_all();
        }

        // Wait on the shared condvar while holding lock_b.
        while *shared_w2.announced.lock().unwrap() < 3 {
            guard = shared_w2.cv.wait(guard).unwrap();
        }

        // Woken and finished.
        drop(guard);
    });

    // Notifier: waits until both waiters have announced, then wakes them
    // while holding each lock the waiters need.
    let n = thread::spawn(move || {
        // Wait until both waiters have announced themselves.
        {
            let mut count = shared_n.announced.lock().unwrap();
            while *count < 2 {
                count = shared_n.cv.wait(count).unwrap();
            }
            // Mark that the notifier has also arrived, so waiters can proceed.
            *count += 1;
        }

        // Acquire each waiter's lock before waking, so waking does not race
        // with the waiters' wait/finish sequence.
        let _ga = lock_a_n.lock().unwrap();
        let _gb = lock_b_n.lock().unwrap();

        // Wake all waiters on the shared condition variable.
        shared_n.cv.notify_all();

        drop(_gb);
        drop(_ga);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    n.join().unwrap();

    println!("DONE done=1");
}
