use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    m: Mutex<State>,
    cv: Condvar,
}

struct State {
    // permit counter: number of waiters that have arrived and are ready to wait
    ready: usize,
    // number of waiters that have been woken
    woken: usize,
    // total waiters expected
    total: usize,
    // whether notifier has fired
    notified: bool,
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(State {
            ready: 0,
            woken: 0,
            total: 2,
            notified: false,
        }),
        cv: Condvar::new(),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let sn = Arc::clone(&shared);

    let w1 = thread::spawn(move || waiter(s1));
    let w2 = thread::spawn(move || waiter(s2));
    let notifier = thread::spawn(move || notifier(sn));

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
}

fn waiter(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();
    // R4: hold the lock while waiting
    // R6: increment permit counter to signal readiness
    state.ready += 1;
    // R2: block until told to proceed
    while !state.notified {
        state = shared.cv.wait(state).unwrap();
    }
    // R9: waiter completes
    state.woken += 1;
    drop(state);
}

fn notifier(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();
    // R6: wait until both waiters are ready to wait
    while state.ready < state.total {
        state = shared.cv.wait(state).unwrap();
    }
    // R5: hold the lock while waking
    state.notified = true;
    // R7: wake every blocked waiter
    shared.cv.notify_all();
    drop(state);
}
