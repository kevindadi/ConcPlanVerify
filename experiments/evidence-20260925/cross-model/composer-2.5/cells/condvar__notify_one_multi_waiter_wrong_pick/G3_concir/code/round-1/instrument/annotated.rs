mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Shared {
    waiters: i32,
    woke: bool,
}

fn w1(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, g12: Arc<Semaphore>) {
    let mut guard = m.lock().unwrap();
    g12.release(1);
    while !guard.woke {
        guard = cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
}

fn w2(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, g12: Arc<Semaphore>) {
    let mut guard = m.lock().unwrap();
    g12.release(1);
    while !guard.woke {
        guard = cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let _gN = gN.acquire();
    let _g12a = g12.acquire();
    let _g12b = g12.acquire();
    let mut guard = m.lock().unwrap();
    guard.woke = true;
    cv.notify_all();
}

fn print_done(m: &Arc<Mutex<Shared>>) {
    let guard = m.lock().unwrap();
    println!("DONE waiters={}", guard.waiters);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared {
        waiters: 2,
        woke: false,
    }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Semaphore::new_named("g12_semaphore0", 0);
    let gN = Semaphore::new_named("gN_semaphore0", 0);

    gN.release(1);

    let m_w1 = Arc::clone(&m);
    let cv_w1 = Arc::clone(&cv);
    let g12_w1 = Arc::clone(&g12);
    let t_w1 = cir_trace::spawn("w1", move || w1(m_w1, cv_w1, g12_w1));

    let m_w2 = Arc::clone(&m);
    let cv_w2 = Arc::clone(&cv);
    let g12_w2 = Arc::clone(&g12);
    let t_w2 = cir_trace::spawn("w2", move || w2(m_w2, cv_w2, g12_w2));

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let t_notifier = cir_trace::spawn("notifier", move || notifier(m_n, cv_n, g12_n, gN_n));

    t_w1.join().unwrap();
    t_w2.join().unwrap();
    t_notifier.join().unwrap();

    print_done(&m);
 cir_trace::finish();}
