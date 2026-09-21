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

    let mut handles = Vec::new();

    // Waiter A
    {
        let shared = Arc::clone(&shared);
        let lock_a = Arc::clone(&lock_a);
        handles.push(thread::spawn(move || {
            let mut guard = lock_a.lock().unwrap();

            // Announce that we are about to wait, while holding our own lock.
            {
                let mut announced = shared.announced.lock().unwrap();
                *announced += 1;
                shared.cv.notify_all();
            }

            // Wait on the shared condition variable while holding our own lock.
            // The notifier will hold lock_a when it wakes us.
            while *shared.announced.lock().unwrap() < 3 {
                guard = shared.cv.wait(guard).unwrap();
            }

            // Woken and finished.
            drop(guard);
        }));
    }

    // Waiter B
    {
        let shared = Arc::clone(&shared);
        let lock_b = Arc::clone(&lock_b);
        handles.push(thread::spawn(move || {
            let mut guard = lock_b.lock().unwrap();

            // Announce that we are about to wait, while holding our own lock.
            {
                let mut announced = shared.announced.lock().unwrap();
                *announced += 1;
                shared.cv.notify_all();
            }

            // Wait on the shared condition variable while holding our own lock.
            // The notifier will hold lock_b when it wakes us.
            while *shared.announced.lock().unwrap() < 3 {
                guard = shared.cv.wait(guard).unwrap();
            }

            // Woken and finished.
            drop(guard);
        }));
    }

    // Notifier
    {
        let shared = Arc::clone(&shared);
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        handles.push(thread::spawn(move || {
            // Wait until both waiters have announced themselves.
            {
                let mut announced = shared.announced.lock().unwrap();
                while *announced < 2 {
                    announced = shared.cv.wait(announced).unwrap();
                }
                // Mark that the notifier has also arrived, so waiters can proceed.
                *announced += 1;
            }

            // Hold each lock that a waiter needs in order to wake and finish.
            let _ga = lock_a.lock().unwrap();
            let _gb = lock_b.lock().unwrap();

            // Wake the waiters.
            shared.cv.notify_all();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
