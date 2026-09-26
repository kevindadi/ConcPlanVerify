mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let sem = Semaphore::new_named("sem_semaphore0", 1);
    let permit = sem.acquire();
    drop(permit);

    let state = Arc::new((Mutex::new_named("state_mutex0", false), Condvar::new_named("state_condvar0")));

    let state_waiter = Arc::clone(&state);
    let waiter = cir_trace::spawn("waiter", move || {
        let (m, cv) = &*state_waiter;
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
    });

    let state_notifier = Arc::clone(&state);
    let notifier = cir_trace::spawn("notifier", move || {
        let (m, cv) = &*state_notifier;
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (m, _cv) = &*state;
    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
