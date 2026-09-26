use std::sync::{Condvar, Mutex};
use std::thread;

fn main() {
    let m = Mutex::new(false);
    let cv = Condvar::new();

    thread::scope(|s| {
        s.spawn(|| waiter(&m, &cv));
        s.spawn(|| notifier(&m, &cv));
    });

    println!("DONE ready=true");
}

fn waiter(m: &Mutex<bool>, cv: &Condvar) {
    let mut seen = m.lock().unwrap();
    while !*seen {
        seen = cv.wait(seen).unwrap();
    }
}

fn notifier(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
}
