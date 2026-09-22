mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared state: lock `m` guards the boolean flag `ready`; `cv` is the condition variable.
    let shared = Arc::new((Mutex::new_named("shared_mutex0", false), Condvar::new_named("shared_condvar0")));

    // Waiter role: checks the flag under the lock, waits only while it is false,
    // and re-checks after every wake. The lock is released while blocked in `wait`.
    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter", move || {
        let (m, cv) = &*waiter_shared;
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
    });

    // Notifier role: sets the flag to true while holding the lock, then signals,
    // then releases the lock. Setting the flag before signaling ensures the
    // notification is never missed, even if the waiter has not started waiting.
    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        let (m, cv) = &*notifier_shared;
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    // Read the final flag value while holding the lock.
    let (m, _) = &*shared;
    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
