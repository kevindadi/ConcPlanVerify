use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared resources: m (lock), cv (condition variable), ready (shared flag)
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    // waiter role
    let waiter = thread::spawn(move || {
        let (m, cv) = &*waiter_shared;
        // Hold the lock while checking the flag
        let mut guard = m.lock().unwrap();
        // Check the flag first; wait only while it is false; re-check after each wake
        while !*guard {
            // wait releases the lock while blocked and reacquires it on wake
            guard = cv.wait(guard).unwrap();
        }
        // The waiter must not pass its wait until the flag is true
        assert!(*guard, "waiter woke up but ready is still false");
    });

    // notifier role
    let notifier = thread::spawn(move || {
        let (m, cv) = &*notifier_shared;
        // Set the shared flag to true while holding the lock
        let mut guard = m.lock().unwrap();
        *guard = true;
        // Signal the condition variable, then release the lock
        cv.notify_one();
        drop(guard);
    });

    // Both roles run at the same time; wait for both to finish
    waiter.join().unwrap();
    notifier.join().unwrap();

    // Verify the shared flag became true in this schedule
    let (m, _) = &*shared;
    let ready = *m.lock().unwrap();
    assert!(ready, "ready must be true in every schedule");

    // Print exactly the required line, then exit
    println!("DONE ready={}", ready);
}
