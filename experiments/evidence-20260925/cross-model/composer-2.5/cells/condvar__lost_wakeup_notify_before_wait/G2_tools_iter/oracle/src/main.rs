mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let sem = Semaphore::new_named("sem_semaphore0", 2);

    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let sem_w = sem.clone();
    let waiter_handle = cir_trace::spawn("waiter", move || {
        let _permit = sem_w.acquire();
        waiter(m_w, cv_w);
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let sem_n = sem.clone();
    let notifier_handle = cir_trace::spawn("notifier", move || {
        let _permit = sem_n.acquire();
        notifier(m_n, cv_n);
    });

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = m.lock().unwrap().ready;
    println!("DONE ready={}", ready);
 cir_trace::finish();}
