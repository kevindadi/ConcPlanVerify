use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let _sem = Semaphore::new(1);
    let ready = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let m_wait = Arc::clone(&ready);
    let cv_wait = Arc::clone(&cv);
    let handle_waiter = thread::spawn(move || waiter(m_wait, cv_wait));

    let m_notify = Arc::clone(&ready);
    let cv_notify = Arc::clone(&cv);
    let handle_notifier = thread::spawn(move || notifier(m_notify, cv_notify));

    handle_waiter.join().unwrap();
    handle_notifier.join().unwrap();

    let guard = ready.lock().unwrap();
    assert!(*guard);
    drop(guard);

    println!("DONE ready=true");
}
