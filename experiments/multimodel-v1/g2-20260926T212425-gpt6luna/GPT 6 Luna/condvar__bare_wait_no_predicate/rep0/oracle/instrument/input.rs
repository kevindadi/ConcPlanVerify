use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
    drop(ready);
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter_thread = thread::spawn(move || waiter(waiter_m, waiter_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier_thread = thread::spawn(move || notifier(notifier_m, notifier_cv));

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={ready}");
}
