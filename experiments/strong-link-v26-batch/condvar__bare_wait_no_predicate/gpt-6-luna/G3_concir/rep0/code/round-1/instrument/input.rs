use std::sync::{Condvar, Mutex};
use std::thread;

fn waiter(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
    drop(ready);
}

fn notifier(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
    drop(ready);
}

fn main() {
    let m = Mutex::new(false);
    let cv = Condvar::new();

    thread::scope(|scope| {
        scope.spawn(|| waiter(&m, &cv));
        scope.spawn(|| notifier(&m, &cv));
    });

    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
}
