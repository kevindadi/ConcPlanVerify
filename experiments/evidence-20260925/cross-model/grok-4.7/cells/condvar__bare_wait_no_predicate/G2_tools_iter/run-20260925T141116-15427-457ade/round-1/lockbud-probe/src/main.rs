use std::sync::{Condvar, Mutex};
use std::thread;

fn main() {
    let m = Mutex::new(false);
    let cv = Condvar::new();

    thread::scope(|s| {
        let waiter = s.spawn(|| {
            let mut ready = m.lock().unwrap();
            while !*ready {
                ready = cv.wait(ready).unwrap();
            }
        });

        let notifier = s.spawn(|| {
            let mut ready = m.lock().unwrap();
            *ready = true;
            cv.notify_one();
            drop(ready);
        });

        waiter.join().unwrap();
        notifier.join().unwrap();
    });

    let ready = *m.lock().unwrap();
    println!("DONE ready={ready}");
}
