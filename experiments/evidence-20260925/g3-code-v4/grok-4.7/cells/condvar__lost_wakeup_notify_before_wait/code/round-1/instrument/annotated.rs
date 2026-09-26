mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter", move || waiter(m_waiter, cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier", move || notifier(m_notifier, cv_notifier));

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = m.lock().unwrap().ready;
    println!("DONE ready={ready}");
 cir_trace::finish();}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    let mut seen = guard.ready;
    while seen == false {
        guard = cv.wait(guard).unwrap();
        seen = guard.ready;
    }
    drop(guard);
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
    drop(guard);
}
