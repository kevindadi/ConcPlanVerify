use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let _ = Semaphore::new(1);

    let m = Arc::new(Mutex::new(Guarded { ready: false }));
    let cv = Arc::new(Condvar::new());

    let m_wait = Arc::clone(&m);
    let cv_wait = Arc::clone(&cv);
    let waiter_handle = thread::spawn(move || waiter(m_wait, cv_wait));

    let m_note = Arc::clone(&m);
    let cv_note = Arc::clone(&cv);
    let notifier_handle = thread::spawn(move || notifier(m_note, cv_note));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = m.lock().unwrap().ready;
    assert!(ready);
    println!("DONE ready=true");
}
