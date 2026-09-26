mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    *guard = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let _sem = Semaphore::new_named("_sem_semaphore0", 1);
    let ready = Arc::new(Mutex::new_named("ready_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_wait = Arc::clone(&ready);
    let cv_wait = Arc::clone(&cv);
    let handle_waiter = cir_trace::spawn("waiter", move || waiter(m_wait, cv_wait));

    let m_notify = Arc::clone(&ready);
    let cv_notify = Arc::clone(&cv);
    let handle_notifier = cir_trace::spawn("notifier", move || notifier(m_notify, cv_notify));

    handle_waiter.join().unwrap();
    handle_notifier.join().unwrap();

    let guard = ready.lock().unwrap();
    assert!(*guard);
    drop(guard);

    println!("DONE ready=true");
 cir_trace::finish();}
