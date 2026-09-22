use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct State {
    permits: usize, // permit counter: one permit per waiter that is ready to wait
    proceed: bool,  // set by the notifier to release the waiters
    waiting: usize, // number of waiters currently blocked on the condition variable
}

struct Shared {
    state: Mutex<State>,
    proceed_cv: Condvar, // waiters block here until told to proceed
    ready_cv: Condvar,   // notifier blocks here until both waiters are ready
}

fn waiter(shared: Arc<Shared>) {
    // R4: the waiter acquires the lock before waiting.
    let mut st = shared.state.lock().unwrap();

    // R6: hand a permit to the notifier, announcing readiness to wait.
    st.permits += 1;
    st.waiting += 1;
    if st.permits == 2 {
        shared.ready_cv.notify_all();
    }

    // R2: block on the shared condition variable until told to proceed.
    // The lock is held while entering the wait and re-acquired on wakeup.
    while !st.proceed {
        st = shared.proceed_cv.wait(st).unwrap();
    }

    st.waiting -= 1;
}

fn notifier(shared: Arc<Shared>) {
    // R5: the notifier takes the lock before waking the waiters.
    let mut st = shared.state.lock().unwrap();

    // R6: only proceed once both waiters have handed over their permit,
    // i.e. both are ready to wait (or already blocked) on the condition.
    while st.permits < 2 {
        st = shared.ready_cv.wait(st).unwrap();
    }

    // R3/R7: wake every blocked waiter, not just one.
    st.proceed = true;
    shared.proceed_cv.notify_all();

    // R5: the lock is released when `st` goes out of scope here.
}

fn main() {
    // R1: shared state for the two waiters and the one notifier.
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            permits: 0,
            proceed: false,
            waiting: 0,
        }),
        proceed_cv: Condvar::new(),
        ready_cv: Condvar::new(),
    });

    let mut handles = Vec::new();

    // R1: start two waiter roles.
    for _ in 0..2 {
        let s = Arc::clone(&shared);
        handles.push(thread::spawn(move || waiter(s)));
    }

    // R1: start one notifier role, running concurrently with the waiters.
    {
        let s = Arc::clone(&shared);
        handles.push(thread::spawn(move || notifier(s)));
    }

    // R8/R9: join every thread; all schedules terminate.
    for h in handles {
        h.join().unwrap();
    }

    // R10: print exactly the required line and exit.
    let st = shared.state.lock().unwrap();
    println!("DONE waiters={}", st.waiting);
}
