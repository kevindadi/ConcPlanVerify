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

    let done1 = sem.acquire();
    let done2 = sem.acquire();

    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let waiter_handle = thread::spawn(move || {
        waiter(&m_w, &cv_w);
        drop(done1);
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let notifier_handle = thread::spawn(move || {
        notifier(&m_n, &cv_n);
        drop(done2);
    });

    let _ = sem.acquire();
    let _ = sem.acquire();

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
}
