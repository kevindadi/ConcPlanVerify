use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let shared = Arc::new((Mutex::new(false), Condvar::new()));
    let (m, cv) = (&shared.0, &shared.1);

    let waiter = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (m, cv) = (&shared.0, &shared.1);
            let mut ready = m.lock().unwrap();
            while !*ready {
                ready = cv.wait_while(ready, |ready| !*ready).unwrap();
            }
            ready
        })
    };

    let notifier = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (m, cv) = (&shared.0, &shared.1);
            let mut ready = m.lock().unwrap();
            *ready = true;
            cv.notify_one();
            drop(ready);
        })
    };

    let ready = *waiter.join().unwrap();
    notifier.join().unwrap();
    println!("DONE ready={}", ready);
}
