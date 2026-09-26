use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct State {
    waiters: i32,
    proceed: bool,
}

fn w1(m: Arc<Mutex<State>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let t12 = g12.acquire();
    let tN = gN.acquire();
    let mut guard = m.lock().unwrap();
    t12.release();
    tN.release();
    while !guard.proceed {
        guard = cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
}

fn w2(m: Arc<Mutex<State>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let t12 = g12.acquire();
    let tN = gN.acquire();
    let mut guard = m.lock().unwrap();
    t12.release();
    tN.release();
    while !guard.proceed {
        guard = cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
}

fn notifier(m: Arc<Mutex<State>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let _ = gN.acquire();
    let _ = gN.acquire();
    let _ = g12.acquire();
    let _ = g12.acquire();
    let mut guard = m.lock().unwrap();
    guard.proceed = true;
    cv.notify_all();
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(State {
        waiters: 2,
        proceed: false,
    }));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);

    let m_w1 = m.clone();
    let cv_w1 = cv.clone();
    let g12_w1 = g12.clone();
    let gN_w1 = gN.clone();
    let h_w1 = thread::spawn(move || w1(m_w1, cv_w1, g12_w1, gN_w1));

    let m_w2 = m.clone();
    let cv_w2 = cv.clone();
    let g12_w2 = g12.clone();
    let gN_w2 = gN.clone();
    let h_w2 = thread::spawn(move || w2(m_w2, cv_w2, g12_w2, gN_w2));

    let m_n = m.clone();
    let cv_n = cv.clone();
    let g12_n = g12.clone();
    let gN_n = gN.clone();
    let h_notifier = thread::spawn(move || notifier(m_n, cv_n, g12_n, gN_n));

    h_w1.join().unwrap();
    h_w2.join().unwrap();
    h_notifier.join().unwrap();

    let guard = m.lock().unwrap();
    println!("DONE waiters={}", guard.waiters);
}
