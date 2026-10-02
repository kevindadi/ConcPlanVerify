use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, permit: Permit) {
    let mut proceed = m.lock().unwrap();
    permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, permit: Permit) {
    let mut proceed = m.lock().unwrap();
    permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn notifier(
    g12: Arc<Semaphore>,
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g_n_permit: Permit,
) {
    let _permit1 = g12.acquire();
    let _permit2 = g12.acquire();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    g_n_permit.release();
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    // Seed permits for the waiters and notifier, then hand them off.
    let g12 = Semaphore::new(2);
    let w1_permit = g12.acquire();
    let w2_permit = g12.acquire();

    let g_n = Semaphore::new(1);
    let g_n_permit = g_n.acquire();

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w1(m, cv, w1_permit))
    };
    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w2(m, cv, w2_permit))
    };
    let notifier_handle = {
        let g12 = Arc::clone(&g12);
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || notifier(g12, m, cv, g_n_permit))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let _permit = g_n.acquire();
    println!("DONE waiters=0");
}
