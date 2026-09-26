mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn waiter(m: &Mutex<Shared>, cv: &Condvar) {
    let mut guard = m.lock().unwrap();
    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(m: &Mutex<Shared>, cv: &Condvar) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let sem = Semaphore::new_named("sem_semaphore0", 2);

    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let sem_w = Arc::clone(&sem);
    let waiter_handle = cir_trace::spawn("waiter", move || {
        let _permit = sem_w.acquire();
        waiter(m_w.as_ref(), cv_w.as_ref());
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let sem_n = Arc::clone(&sem);
    let notifier_handle = cir_trace::spawn("notifier", move || {
        let _permit = sem_n.acquire();
        notifier(m_n.as_ref(), cv_n.as_ref());
    });

    let _wait_w = sem.acquire();
    let _wait_n = sem.acquire();

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
