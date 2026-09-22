use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct State {
    proceed: bool,
    ready: usize,
    waiting: usize,
}

fn main() {
    let state = Arc::new(Mutex::new(State {
        proceed: false,
        ready: 0,
        waiting: 0,
    }));
    let proceed_cv = Arc::new(Condvar::new());
    let ready_cv = Arc::new(Condvar::new());

    let mut handles = Vec::new();

    // Two waiter roles.
    for _ in 0..2 {
        let state = Arc::clone(&state);
        let proceed_cv = Arc::clone(&proceed_cv);
        let ready_cv = Arc::clone(&ready_cv);
        handles.push(thread::spawn(move || {
            let mut guard = state.lock().unwrap();
            guard.ready += 1;
            guard.waiting += 1;
            // Signal the notifier that one more waiter is ready to wait.
            ready_cv.notify_all();
            // Hold the lock while waiting on the condition variable.
            while !guard.proceed {
                guard = proceed_cv.wait(guard).unwrap();
            }
            guard.waiting -= 1;
        }));
    }

    // One notifier role.
    {
        let state = Arc::clone(&state);
        let proceed_cv = Arc::clone(&proceed_cv);
        let ready_cv = Arc::clone(&ready_cv);
        handles.push(thread::spawn(move || {
            // Take the lock before waking the waiters.
            let mut guard = state.lock().unwrap();
            // Wait until the permit counter shows both waiters are ready.
            while guard.ready < 2 {
                guard = ready_cv.wait(guard).unwrap();
            }
            guard.proceed = true;
            // Wake every blocked waiter, not just one.
            proceed_cv.notify_all();
            // Release the lock afterwards.
            drop(guard);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let guard = state.lock().unwrap();
    println!("DONE waiters={}", guard.waiting);
}
