use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12_permit: concir_sync::Permit,
    g_n: Arc<Semaphore>,
) {
    let guard = m.lock().unwrap();
    g12_permit.release();
    let guard = cv.wait(guard).unwrap();
    let _g_n_permit = g_n.acquire();
    drop(guard);
}

fn w2(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12_permit: concir_sync::Permit,
    g_n: Arc<Semaphore>,
) {
    let guard = m.lock().unwrap();
    g12_permit.release();
    let guard = cv.wait(guard).unwrap();
    let _g_n_permit = g_n.acquire();
    drop(guard);
}

fn notifier(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    g_n_permit_1: concir_sync::Permit,
    g_n_permit_2: concir_sync::Permit,
) {
    let _g12_permit_1 = g12.acquire();
    let _g12_permit_2 = g12.acquire();
    let _guard = m.lock().unwrap();
    g_n_permit_1.release();
    g_n_permit_2.release();
    cv.notify_all();
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(2);
    let g_n = Semaphore::new(2);

    let g12_permit_1 = g12.acquire();
    let g12_permit_2 = g12.acquire();
    let g_n_permit_1 = g_n.acquire();
    let g_n_permit_2 = g_n.acquire();

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g_n = Arc::clone(&g_n);
        thread::spawn(move || w1(m, cv, g12_permit_1, g_n))
    };
    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g_n = Arc::clone(&g_n);
        thread::spawn(move || w2(m, cv, g12_permit_2, g_n))
    };
    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        thread::spawn(move || {
            notifier(m, cv, g12, g_n_permit_1, g_n_permit_2)
        })
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
}
