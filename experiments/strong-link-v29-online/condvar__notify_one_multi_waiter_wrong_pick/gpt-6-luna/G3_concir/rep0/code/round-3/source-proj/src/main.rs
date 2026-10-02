use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    gN: &'static Arc<Semaphore>,
    g12_permit: Permit,
) {
    let mut guard = m.lock().unwrap();
    if !*guard {
        g12_permit.release();
        while !*guard {
            guard = cv.wait(guard).unwrap();
        }
    }
    drop(guard);
    let _permit = gN.acquire();
}

fn w2(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    gN: &'static Arc<Semaphore>,
    g12_permit: Permit,
) {
    let mut guard = m.lock().unwrap();
    if !*guard {
        g12_permit.release();
        while !*guard {
            guard = cv.wait(guard).unwrap();
        }
    }
    drop(guard);
    let _permit = gN.acquire();
}

fn notifier(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: &'static Arc<Semaphore>,
    gN_permit1: Permit,
    gN_permit2: Permit,
) {
    let _ready1 = g12.acquire();
    let _ready2 = g12.acquire();

    let mut guard = m.lock().unwrap();
    *guard = true;
    cv.notify_all();
    gN_permit1.release();
    gN_permit2.release();
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    // Hold the initial permits so the workers and notifier can release them
    // at the corresponding points in the design.
    let g12: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(2)));
    let gN: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(2)));

    let g12_permit1 = g12.acquire();
    let g12_permit2 = g12.acquire();
    let gN_permit1 = gN.acquire();
    let gN_permit2 = gN.acquire();

    let t1 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w1(m, cv, gN, g12_permit1))
    };

    let t2 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w2(m, cv, gN, g12_permit2))
    };

    let tn = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || notifier(m, cv, g12, gN_permit1, gN_permit2))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    println!("DONE waiters=0");
}
