mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        announced: Mutex::new_named("shared_mutex0", 0),
        cv: Condvar::new_named("shared_condvar0"),
        lock_a: Mutex::new_named("shared_mutex1", ()),
        lock_b: Mutex::new_named("shared_mutex2", ()),
    });

    let s1 = Arc::clone(&shared);
    let waiter_a = cir_trace::spawn("waiter_a", move || {
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
    let waiter_b = cir_trace::spawn("waiter_b", move || {
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
    let notifier = cir_trace::spawn("notifier", move || {
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
 cir_trace::finish();}
