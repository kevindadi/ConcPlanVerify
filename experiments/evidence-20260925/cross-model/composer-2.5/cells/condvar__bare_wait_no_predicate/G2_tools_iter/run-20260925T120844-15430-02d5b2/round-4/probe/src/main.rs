use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let m = Arc::new(Mutex::new(Shared { ready: false }));
    let cv = Arc::new(Condvar::new());
    let sem = Semaphore::new(2);

    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let sem_w = Arc::clone(&sem);
    let waiter_handle = thread::spawn(move || {
        let _permit = sem_w.acquire();
        waiter(m_w.as_ref(), cv_w.as_ref());
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let sem_n = Arc::clone(&sem);
    let notifier_handle = thread::spawn(move || {
        let _permit = sem_n.acquire();
        notifier(m_n.as_ref(), cv_n.as_ref());
    });

    let _wait_w = sem.acquire();
    let _wait_n = sem.acquire();

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
}
