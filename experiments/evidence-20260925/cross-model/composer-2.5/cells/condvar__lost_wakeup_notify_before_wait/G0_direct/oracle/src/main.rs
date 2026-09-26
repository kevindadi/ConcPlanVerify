mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Guarded {
    ready: bool,
}

fn waiter(m: Arc<Mutex<Guarded>>, cv: Arc<Condvar>) {
    let mut lock = m.lock().unwrap();
    while !lock.ready {
        lock = cv.wait(lock).unwrap();
    }
}

fn notifier(m: Arc<Mutex<Guarded>>, cv: Arc<Condvar>) {
    let mut lock = m.lock().unwrap();
    lock.ready = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let _ = Semaphore::new_named("res_semaphore0", 1);

    let m = Arc::new(Mutex::new_named("m_mutex0", Guarded { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_wait = Arc::clone(&m);
    let cv_wait = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter", move || waiter(m_wait, cv_wait));

    let m_note = Arc::clone(&m);
    let cv_note = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier", move || notifier(m_note, cv_note));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = m.lock().unwrap().ready;
    assert!(ready);
    println!("DONE ready=true");
 cir_trace::finish();}
