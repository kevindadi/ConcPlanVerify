mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // One lock, one condition variable, one boolean flag guarded by the lock.
    let shared = Arc::new((Mutex::new_named("shared_mutex0", false), Condvar::new_named("shared_condvar0")));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    // Waiter role: check the flag under the lock, wait only while it is
    // false, and re-check after every wake. The lock is released while
    // blocked in `wait`.
    let waiter = cir_trace::spawn("waiter", move || {
        let (lock, cvar) = &*waiter_shared;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
    });

    // Notifier role: set the flag to true while holding the lock, signal the
    // condition variable, then release the lock.
    let notifier = cir_trace::spawn("notifier", move || {
        let (lock, cvar) = &*notifier_shared;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _) = &*shared;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
