mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn main() { cir_trace::init();
    let m = Arc::new((Mutex::new_named("m_mutex0", Shared { ready: false }), Condvar::new_named("m_condvar0")));

    let m_waiter = Arc::clone(&m);
    let waiter = cir_trace::spawn("waiter", move || {
        let (lock, cv) = &*m_waiter;
        let mut guard = lock.lock().unwrap();
        while !guard.ready {
            guard = cv.wait(guard).unwrap();
        }
        drop(guard);
    });

    let m_notifier = Arc::clone(&m);
    let notifier = cir_trace::spawn("notifier", move || {
        let (lock, cv) = &*m_notifier;
        let mut guard = lock.lock().unwrap();
        guard.ready = true;
        cv.notify_one();
        drop(guard);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cv) = &*m;
    let guard = lock.lock().unwrap();
    println!("DONE ready={}", guard.ready);
 cir_trace::finish();}
