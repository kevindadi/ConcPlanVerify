use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();

    let permit = g12.acquire();
    permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
    drop(proceed);

    let permit = gN.acquire();
    permit.release();
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();

    let permit = g12.acquire();
    permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
    drop(proceed);

    let permit = gN.acquire();
    permit.release();
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let permit = g12.acquire();
    permit.release();
    let permit = g12.acquire();
    permit.release();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    let permit = gN.acquire();
    permit.release();
    let permit = gN.acquire();
    permit.release();
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone
