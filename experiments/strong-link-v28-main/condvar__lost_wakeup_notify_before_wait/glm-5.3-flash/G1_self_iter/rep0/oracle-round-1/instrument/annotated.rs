mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R2: one lock (m), one condition variable (cv), one boolean flag (ready) guarded by m.
    let shared = Arc::new((Mutex::new_named("shared_mutex0#195", false), Condvar::new_named("shared_condvar0#216")));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    // R1: main starts waiter and notifier running at the same time.
    let waiter_handle = cir_trace::spawn("waiter#412", move || waiter(waiter_shared));
    let notifier_handle = cir_trace::spawn("notifier#484", move || notifier(notifier_shared));

    // R8: join both roles so every schedule terminates with both finished.
    waiter_handle.join().expect("waiter thread panicked");
    notifier_handle.join().expect("notifier thread panicked");
 cir_trace::finish();}

fn waiter(shared: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    // R7: acquire the lock to read the flag.
    let guard = m.lock().unwrap();
    // R4: check the flag while holding the lock; wait only while it is false;
    // re-check after every wake (the `while` loop handles spurious wakeups
    // and the missed-signal race in R5).
    let mut guard = cv.wait_while(guard, |ready| !*ready).unwrap();
    // R7: the lock is released when `guard` is dropped; the waiter was not
    // holding it while blocked (Condvar::wait_while releases it internally).
    drop(guard);

    // R9/R10: the flag is true here in every schedule.
    println!("DONE ready=true");
}

fn notifier(shared: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    // R3: set the flag to true while holding the lock...
    let mut guard = m.lock().unwrap();
    *guard = true;
    // R6: the flag is true before the signal is issued, so a waiter that is
    // about to wait (or already waiting) can never miss the notification.
    cv.notify_one();
    // R3: ...and then releases the lock (guard dropped at end of scope).
}
