mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let shared = Arc::new((Mutex::new_named("shared_mutex0#102", false), Condvar::new_named("shared_condvar0#123")));

    let (m, cv) = (Arc::clone(&shared), Arc::clone(&shared));
    let waiter = cir_trace::spawn("waiter#212", move || {
        let (lock, cv) = (&*m.0, &*cv.1);
        let mut guard = lock.lock().unwrap();
        while !*guard {
            guard = cv.wait(guard).unwrap();
        }
        assert!(*guard);
    });

    let notifier = cir_trace::spawn("notifier#456", move || {
        let (lock, cv) = (&*shared.0, &*shared.1);
        let mut guard = lock.lock().unwrap();
        *guard = true;
        cv.notify_one();
        drop(guard);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = *shared.0.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
