mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
) {
    let _ = gN.acquire();
    let ready = g12.acquire();
    let guard = m.lock().unwrap();
    ready.release();
    let _guard = cv.wait(guard).unwrap();
}

fn w2(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
) {
    let _ = gN.acquire();
    let ready = g12.acquire();
    let guard = m.lock().unwrap();
    ready.release();
    let _guard = cv.wait(guard).unwrap();
}

fn notifier(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, _gN: Arc<Semaphore>) {
    let _ = g12.acquire();
    let _ = g12.acquire();
    let guard = m.lock().unwrap();
    cv.notify_all();
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Semaphore::new_named("g12_semaphore0", 2);
    let gN = Semaphore::new_named("gN_semaphore0", 2);

    let m1 = m.clone();
    let cv1 = cv.clone();
    let g12_1 = g12.clone();
    let gN_1 = gN.clone();
    let h1 = cir_trace::spawn("w1", move || w1(m1, cv1, g12_1, gN_1));

    let m2 = m.clone();
    let cv2 = cv.clone();
    let g12_2 = g12.clone();
    let gN_2 = gN.clone();
    let h2 = cir_trace::spawn("w2", move || w2(m2, cv2, g12_2, gN_2));

    let m3 = m.clone();
    let cv3 = cv.clone();
    let g12_3 = g12.clone();
    let gN_3 = gN.clone();
    let h3 = cir_trace::spawn("notifier", move || notifier(m3, cv3, g12_3, gN_3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE waiters=0");
 cir_trace::finish();}
