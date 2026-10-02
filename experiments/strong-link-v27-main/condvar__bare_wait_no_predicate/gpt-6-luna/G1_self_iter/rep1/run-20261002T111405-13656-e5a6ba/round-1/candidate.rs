use std::sync::{Arc, Condvar, Mutex};

struct State {
    ready: bool,
}

fn waiter(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();

    while !state.ready {
        state = cv.wait(state).unwrap();
    }
}

fn notifier(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();
    state.ready = true;
    cv.notify_one();
}

fn main() {
    let m = Arc::new(Mutex::new(State { ready: false }));
    let cv = Arc::new(Condvar::new());

    let waiter_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        std::thread::spawn(move || waiter(m, cv))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        std::thread::spawn(move || notifier(m, cv))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let state = m.lock().unwrap();
    println!("DONE ready={}", state.ready);
}
