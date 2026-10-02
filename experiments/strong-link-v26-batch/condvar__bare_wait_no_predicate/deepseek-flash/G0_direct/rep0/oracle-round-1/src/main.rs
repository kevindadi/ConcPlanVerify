mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#132", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#194"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter#299", move || waiter(m_waiter, cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier#454", move || notifier(m_notifier, cv_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let shared = m.lock().unwrap();
    println!("DONE ready={}", shared.ready);
 cir_trace::finish();}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut shared = m.lock().unwrap();

    while !shared.ready {
        shared = cv.wait(shared).unwrap();
    }
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut shared = m.lock().unwrap();

    shared.ready = true;
    cv.notify_one();
}
