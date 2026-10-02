use std::sync::{Arc, Condvar, Mutex};

fn waiter(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    let mut ready = *guard;
    while !ready {
        guard = cv.wait(guard).unwrap();
        ready = *guard;
    }
    drop(guard);
}

fn notifier(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    *guard = true;
    cv.notify_all();
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = std::thread::spawn(move || waiter(&m_waiter, &cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = std::thread::spawn(move || notifier(&m_notifier, &cv_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
}
