mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct State {
    ready: bool,
}

fn waiter(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();

    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#439", State { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#500"));

    let waiter_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("waiter#609", move || waiter(m, cv))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("notifier#756", move || notifier(m, cv))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = m.lock().unwrap().ready;
    println!("DONE ready={ready}");
 cir_trace::finish();}
