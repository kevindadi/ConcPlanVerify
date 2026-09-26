use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();

    while !*guard {
        guard = cv.wait(guard).unwrap();
    }

    drop(guard);
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    *guard = true;
    cv.notify_one();
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let waiter_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || waiter(m, cv))
    };

    let notifier_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || notifier(m, cv))
    };

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE ready=true");
}
