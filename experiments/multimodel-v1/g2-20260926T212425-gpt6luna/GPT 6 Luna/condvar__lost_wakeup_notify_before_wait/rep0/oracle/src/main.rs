mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct State {
    ready: bool,
}

struct Shared {
    m: Mutex<State>,
    cv: Condvar,
}

fn main() { cir_trace::init();
    let _semaphore = Semaphore::new_named("_semaphore_semaphore0", 1);

    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", State { ready: false }),
        cv: Condvar::new_named("shared_condvar0"),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut state = waiter_shared.m.lock().unwrap();
        while !state.ready {
            state = waiter_shared.cv.wait(state).unwrap();
        }
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut state = notifier_shared.m.lock().unwrap();
        state.ready = true;
        notifier_shared.cv.notify_one();
        drop(state);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = shared.m.lock().unwrap().ready;
    println!("DONE ready={ready}");
 cir_trace::finish();}
