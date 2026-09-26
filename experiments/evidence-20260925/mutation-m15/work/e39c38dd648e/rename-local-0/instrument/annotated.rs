mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn main() { cir_trace::init();
    let m_kept = Arc::new(Mutex::new_named("m_kept_mutex0", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_waiter = Arc::clone(&m_kept);
    let cv_waiter = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut guard = m_waiter.lock().unwrap();
        while !guard.ready {
            guard = cv_waiter.wait(guard).unwrap();
        }
        drop(guard);
    });

    let m_notifier = Arc::clone(&m_kept);
    let cv_notifier = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut guard = m_notifier.lock().unwrap();
        guard.ready = true;
        cv_notifier.notify_one();
        drop(guard);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let guard = m_kept.lock().unwrap();
    println!("DONE ready={}", guard.ready);
 cir_trace::finish();}
