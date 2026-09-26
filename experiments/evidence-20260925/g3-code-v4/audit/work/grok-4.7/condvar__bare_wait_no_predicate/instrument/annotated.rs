mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0", false);
    let cv = Condvar::new_named("cv_condvar0");

    thread::scope(|s| {
        s.spawn(|| waiter(&m, &cv));
        s.spawn(|| notifier(&m, &cv));
    });

    println!("DONE ready=true");
 cir_trace::finish();}

fn waiter(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
}
