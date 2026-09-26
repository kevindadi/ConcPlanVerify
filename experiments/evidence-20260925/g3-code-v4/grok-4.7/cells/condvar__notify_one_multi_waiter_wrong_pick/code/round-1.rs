use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use concir_sync::Semaphore;

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    let m_w1 = Arc::clone(&m);
    let cv_w1 = Arc::clone(&cv);
    let g12_w1 = Arc::clone(&g12);
    let gN_w1 = Arc::clone(&gN);

    let m_w2 = Arc::clone(&m);
    let cv_w2 = Arc::clone(&cv);
    let g12_w2 = Arc::clone(&g12);
    let gN_w2 = Arc::clone(&gN);

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);

    let w1_handle = thread::spawn(move || w1(m_w1, cv_w1, g12_w1, gN_w1));
    let w2_handle = thread::spawn(move || w2(m_w2, cv_w2, g12_w2, gN_w2));
    let notifier_handle = thread::spawn(move || notifier(m_n, cv_n, g12_n, gN_n));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
}

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut guard = m.lock().unwrap();
    g12.signal();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    drop(guard);
    gN.signal();
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut guard = m.lock().unwrap();
    g12.signal();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    drop(guard);
    gN.signal();
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    std::mem::forget(g12.acquire());
    std::mem::forget(g12.acquire());
    {
        let mut guard = m.lock().unwrap();
        *guard = true;
        cv.notify_one();
        cv.notify_one();
    }
    std::mem::forget(gN.acquire());
    std::mem::forget(gN.acquire());
}
