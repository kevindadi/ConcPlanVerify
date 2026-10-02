use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, permit: concir_sync::Permit) {
    let mut proceed = m.lock().unwrap();
    permit.release();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, permit: concir_sync::Permit) {
    let mut proceed = m.lock().unwrap();
    permit.release();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn notifier(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    g_n_permit: concir_sync::Permit,
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

    // Keep the semaphores alive for the full lifetime of permits borrowed
    // from them and moved into the worker threads.
    let g12: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(2)));
    let g_n: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(1)));

    let w1_permit = g12.acquire();
    let w2_permit = g12.acquire();
    let g_n_permit = g_n.acquire();

    let t1 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w1(m, cv, w1_permit))
    };

    let t2 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w2(m, cv, w2_permit))
    };

    let tn = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(g12);
        thread::spawn(move || notifier(m, cv, g12, g_n_permit))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    let _g_n_permit = g_n.acquire();
    println!("DONE waiters=0");
}
