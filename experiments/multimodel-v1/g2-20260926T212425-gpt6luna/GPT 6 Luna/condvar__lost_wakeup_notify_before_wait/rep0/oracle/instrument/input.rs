use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct State {
    ready: bool,
}

struct Shared {
    m: Mutex<State>,
    cv: Condvar,
}

fn main() {
    let _semaphore = Semaphore::new(1);

    let shared = Arc::new(Shared {
        m: Mutex::new(State { ready: false }),
        cv: Condvar::new(),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let mut state = waiter_shared.m.lock().unwrap();
        while !state.ready {
            state = waiter_shared.cv.wait(state).unwrap();
        }
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let mut state = notifier_shared.m.lock().unwrap();
        state.ready = true;
        notifier_shared.cv.notify_one();
        drop(state);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = shared.m.lock().unwrap().ready;
    println!("DONE ready={ready}");
}
