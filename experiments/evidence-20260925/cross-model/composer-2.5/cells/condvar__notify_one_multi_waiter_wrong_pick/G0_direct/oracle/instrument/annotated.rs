mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", State {
        waiters: 2,
        proceed: false,
    }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Semaphore::new_named("g12_semaphore0", 2);
    let gN = Semaphore::new_named("gN_semaphore0", 2);

    let m_w1 = m.clone();
    let cv_w1 = cv.clone();
    let g12_w1 = g12.clone();
    let gN_w1 = gN.clone();
    let h_w1 = cir_trace::spawn("w1", move || w1(m_w1, cv_w1, g12_w1, gN_w1));

    let m_w2 = m.clone();
    let cv_w2 = cv.clone();
    let g12_w2 = g12.clone();
    let gN_w2 = gN.clone();
    let h_w2 = cir_trace::spawn("w2", move || w2(m_w2, cv_w2, g12_w2, gN_w2));

    let m_n = m.clone();
    let cv_n = cv.clone();
    let g12_n = g12.clone();
    let gN_n = gN.clone();
    let h_notifier = cir_trace::spawn("notifier", move || notifier(m_n, cv_n, g12_n, gN_n));

    h_w1.join().unwrap();
    h_w2.join().unwrap();
    h_notifier.join().unwrap();

    let guard = m.lock().unwrap();
    println!("DONE waiters={}", guard.waiters);
 cir_trace::finish();}
