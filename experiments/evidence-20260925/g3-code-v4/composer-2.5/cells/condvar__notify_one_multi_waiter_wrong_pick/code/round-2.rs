use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use concir_sync::Semaphore;

fn w1(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
) {
    let mut guard = m.lock().unwrap();
    Semaphore::release(&g12);
    guard = cv.wait(guard).unwrap();
    let permit = gN.acquire();
    std::mem::forget(permit);
    drop(guard);
}

fn w2(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
) {
    let mut guard = m.lock().unwrap();
    Semaphore::release(&g12);
    guard = cv.wait(guard).unwrap();
    let permit = gN.acquire();
    std::mem::forget(permit);
    drop(guard);
}

fn notifier(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
) {
    let p12_1 = g12.acquire();
    let p12_2 = g12.acquire();
    let guard = m.lock().unwrap();
    cv.notify_all();
    Semaphore::release(&gN);
    Semaphore::release(&gN);
    drop(guard);
    std::mem::forget(p12_1);
    std::mem::forget(p12_2);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    let m_w1 = Arc::clone(&m);
    let cv_w1 = Arc::clone(&cv);
    let g12_w1 = Arc::clone(&g12);
    let gN_w1 = Arc::clone(&gN);
    let h1 = thread::spawn(move || w1(m_w1, cv_w1, g12_w1, gN_w1));

    let m_w2 = Arc::clone(&m);
    let cv_w2 = Arc::clone(&cv);
    let g12_w2 = Arc::clone(&g12);
    let gN_w2 = Arc::clone(&gN);
    let h2 = thread::spawn(move || w2(m_w2, cv_w2, g12_w2, gN_w2));

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let h3 = thread::spawn(move || notifier(m_n, cv_n, g12_n, gN_n));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE waiters=0");
}
