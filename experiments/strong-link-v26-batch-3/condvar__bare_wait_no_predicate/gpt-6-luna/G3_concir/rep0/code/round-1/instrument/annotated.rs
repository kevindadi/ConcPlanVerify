mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

struct State {
    ready: bool,
}

fn waiter(m: &Mutex<State>, cv: &Condvar) {
    let mut guard = m.lock().unwrap();

    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }

    drop(guard);
}

fn notifier(m: &Mutex<State>, cv: &Condvar) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0#444", State { ready: false });
    let cv = Condvar::new_named("cv_condvar0#495");

    thread::scope(|scope| {
        let waiter_handle = scope.spawn(|| waiter(&m, &cv));
        let notifier_handle = scope.spawn(|| notifier(&m, &cv));

        waiter_handle.join().unwrap();
        notifier_handle.join().unwrap();
    });

    println!("DONE ready={}", m.lock().unwrap().ready);
 cir_trace::finish();}
