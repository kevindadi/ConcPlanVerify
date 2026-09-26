use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct State {
    ready: bool,
}

fn waiter(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }
    drop(guard);
}

fn notifier(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(State { ready: false }));
    let cv = Arc::new(Condvar::new());

    let waiter_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || waiter(m, cv))
    };
    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || notifier(m, cv))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let guard = m.lock().unwrap();
    println!("DONE ready={}", guard.ready);
}
