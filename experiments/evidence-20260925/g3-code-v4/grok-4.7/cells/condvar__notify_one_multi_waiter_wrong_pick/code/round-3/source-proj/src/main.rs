use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use concir_sync::Permit;
use concir_sync::Semaphore;

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);

    thread::scope(|scope| {
        let g12_w1 = g12.acquire();
        let g12_w2 = g12.acquire();
        let gN_w1 = gN.acquire();
        let gN_w2 = gN.acquire();

        let m_w1 = Arc::clone(&m);
        let cv_w1 = Arc::clone(&cv);
        let m_w2 = Arc::clone(&m);
        let cv_w2 = Arc::clone(&cv);
        let m_n = Arc::clone(&m);
        let cv_n = Arc::clone(&cv);
        let g12_n = Arc::clone(&g12);
        let gN_n = Arc::clone(&gN);

        let w1_handle = scope.spawn(move || w1(m_w1, cv_w1, g12_w1, gN_w1));
        let w2_handle = scope.spawn(move || w2(m_w2, cv_w2, g12_w2, gN_w2));
        let notifier_handle = scope.spawn(move || notifier(m_n, cv_n, g12_n, gN_n));

        w1_handle.join().unwrap();
        w2_handle.join().unwrap();
        notifier_handle.join().unwrap();
    });

    println!("DONE waiters=0");
}

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Permit, gN: Permit) {
    let mut guard = m.lock().unwrap();
    g12.release();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    drop(guard);
    gN.release();
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Permit, gN: Permit) {
    let mut guard = m.lock().unwrap();
    g12.release();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    drop(guard);
    gN.release();
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let _g12_a = std::mem::ManuallyDrop::new(g12.acquire());
    let _g12_b = std::mem::ManuallyDrop::new(g12.acquire());
    {
        let mut guard = m.lock().unwrap();
        *guard = true;
        cv.notify_one();
        cv.notify_one();
    }
    let _gN_a = std::mem::ManuallyDrop::new(gN.acquire());
    let _gN_b = std::mem::ManuallyDrop::new(gN.acquire());
}
