use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};

/// The CIR allocates `g12` and `gN` with count 0; a waiter needs to own a
/// permit in order to perform its `semaphore_release`.  Those permits are
/// created here once and handed to the waiter threads, so the semaphore object
/// must outlive every spawned thread.  Leaking it makes it `'static`, which is
/// what `std::thread::spawn` requires for captured data.
fn leaked_semaphore(permits: usize) -> &'static Semaphore {
    let arc: Arc<Semaphore> = Semaphore::new(permits);
    &**Box::leak(Box::new(arc))
}

fn main() {
    // main::m and main::cv
    let m: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let cv: Arc<Condvar> = Arc::new(Condvar::new());

    // main::g12 and main::gN.  Both permits of each semaphore are taken up
    // front so that, once the threads start, the counters are empty and the
    // notifier blocks until the waiters hand their permits back.
    let g12: &'static Semaphore = leaked_semaphore(2);
    let gN: &'static Semaphore = leaked_semaphore(2);

    let p12_w1 = g12.acquire();
    let pN_w1 = gN.acquire();
    let p12_w2 = g12.acquire();
    let pN_w2 = gN.acquire();

    // main::w1
    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let w1 = std::thread::spawn(move || {
        let mut guard = m1.lock().unwrap(); // mutex_lock main::m
        p12_w1.release();                   // semaphore_release main::g12
        guard = cv1.wait(guard).unwrap();   // condvar_wait main::cv (on main::m)
        drop(guard);                        // mutex_unlock main::m
        pN_w1.release();                    // semaphore_release main::gN
    });

    // main::w2
    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let w2 = std::thread::spawn(move || {
        let mut guard = m2.lock().unwrap(); // mutex_lock main::m
        p12_w2.release();                   // semaphore_release main::g12
        guard = cv2.wait(guard).unwrap();   // condvar_wait main::cv (on main::m)
        drop(guard);                        // mutex_unlock main::m
        pN_w2.release();                    // semaphore_release main::gN
    });

    // main::notifier
    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let notifier = std::thread::spawn(move || {
        let q12_a = g12.acquire();          // semaphore_acquire main::g12
        let q12_b = g12.acquire();          // semaphore_acquire main::g12
        let guard = m3.lock().unwrap();     // mutex_lock main::m
        cv3.notify_all();                   // condvar_notify_all main::cv
        drop(guard);                        // mutex_unlock main::m
        let qN_a = gN.acquire();            // semaphore_acquire main::gN
        let qN_b = gN.acquire();            // semaphore_acquire main::gN
        // A CIR acquire consumes its permit; never hand these back.
        std::mem::forget(q12_a);
        std::mem::forget(q12_b);
        std::mem::forget(qN_a);
        std::mem::forget(qN_b);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
}
