mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R2: one lock (m), one condition variable (cv), and one boolean flag (ready)
    // guarded by the lock, shared between the waiter and the notifier.
    let shared = Arc::new((Mutex::new_named("shared_mutex0#257", false), Condvar::new_named("shared_condvar0#278")));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    // R1: waiter and notifier run at the same time as concurrent tasks.
    let waiter = cir_trace::spawn("waiter#471", move || {
        let (m, cv) = &*waiter_shared;

        // R4: check the flag while holding the lock.
        let mut ready = m.lock().unwrap();

        // R4: wait only while the flag is false; re-check after every wake.
        // R5: if the notifier already set the flag before we started waiting,
        //     the loop body never runs and we complete immediately.
        while !*ready {
            // R7: the lock is released while blocked and reacquired on wake.
            ready = cv.wait(ready).unwrap();
        }
    });

    let notifier = cir_trace::spawn("notifier#1043", move || {
        let (m, cv) = &*notifier_shared;

        // R3: set the flag to true while holding the lock...
        let mut ready = m.lock().unwrap();
        *ready = true;

        // R6: the flag is true before the signal is issued, so a waiting
        // role can never miss the notification.
        cv.notify_one();

        // R3: ...and then release the lock (drop at end of scope).
    });

    // R8: join both tasks; every interleaving terminates because the waiter
    // only waits while the flag is false and the flag is set under the lock.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R9: the flag is true in every schedule by this point.
    let (m, _cv) = &*shared;
    let ready = m.lock().unwrap();

    // R10: print exactly the required line and exit.
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
