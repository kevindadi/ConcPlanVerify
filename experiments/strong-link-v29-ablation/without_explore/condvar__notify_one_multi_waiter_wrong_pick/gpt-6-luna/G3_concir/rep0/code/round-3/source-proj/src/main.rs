use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12_permit: Permit,
    g_n: Arc<Semaphore>,
) {
    let mut proceed = m.lock().unwrap();
    g12_permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    drop(proceed);
    let _permit = g_n.acquire();
}

fn w2(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12_permit: Permit,
    g_n: Arc<Semaphore>,
) {
    let mut proceed = m.lock().unwrap();
    g12_permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    drop(proceed);
    let _permit = g_n.acquire();
}

fn notifier(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    g_n_permit1: Permit,
    g_n_permit2: Permit,
) {
    let _permit1 = g12.acquire();
    let _permit2 = g12.acquire();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    g_n_permit1.release();
    g_n_permit2.release();
    drop(proceed);
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    // Reserve the permits so the semaphores begin with no available permits
    // when the worker threads start.
    let g12 = Semaphore::new(2);
    let g_n = Semaphore::new(2);
    let g12_permit1 = g12.acquire();
    let g12_permit2 = g12.acquire();
    let g_n_permit1 = g_n.acquire();
    let g_n_permit2 = g_n.acquire();

    let w1_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g_n = Arc::clone(&g_n);
        thread::spawn(move || w1(m, cv, g12_permit1, g_n))
    };

    let w2_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g_n = Arc::clone(&g_n);
        thread::spawn(move || w2(m, cv, g12_permit2, g_n))
    };

    let notifier_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        thread::spawn(move || {
            notifier(m, cv, g12, g_n_permit1, g_n_permit2)
        })
    };

    w1_thread.join().unwrap();
    w2_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE waiters=0");
}
