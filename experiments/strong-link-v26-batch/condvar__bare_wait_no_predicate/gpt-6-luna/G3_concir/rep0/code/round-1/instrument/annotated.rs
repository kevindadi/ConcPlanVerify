mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

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

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0#395", false);
    let cv = Condvar::new_named("cv_condvar0#429");

    thread::scope(|scope| {
        scope.spawn(|| waiter(&m, &cv));
        scope.spawn(|| notifier(&m, &cv));
    });

    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
