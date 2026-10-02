mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let shared = Arc::new((Mutex::new_named("shared_mutex0#102", false), Condvar::new_named("shared_condvar0#123")));
    let (m, cv) = (&shared.0, &shared.1);

    let waiter = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#244", move || {
            let (m, cv) = (&shared.0, &shared.1);
            let mut ready = m.lock().unwrap();
            while !*ready {
                ready = cv.wait_while(ready, |ready| !*ready).unwrap();
            }
            ready
        })
    };

    let notifier = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#587", move || {
            let (m, cv) = (&shared.0, &shared.1);
            let mut ready = m.lock().unwrap();
            *ready = true;
            cv.notify_one();
            drop(ready);
        })
    };

    let ready = *waiter.join().unwrap();
    notifier.join().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
