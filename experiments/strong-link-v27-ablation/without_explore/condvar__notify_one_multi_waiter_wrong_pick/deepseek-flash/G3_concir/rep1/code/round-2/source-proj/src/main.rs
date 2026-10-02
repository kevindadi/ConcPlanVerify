use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};

fn main() {
    // main::m and main::cv
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());

    // main::g12 and main::gN are signalling semaphores: a CIR
    // `semaphore_release` is "a permit is handed back", and a permit can only
    // be handed back by the thread that owns it.  Each waiter owns one permit
    // of g12 and one permit of gN; both permits of both semaphores are taken
    // here, before any thread exists, so the notifier finds the semaphores
    // empty and cannot run past the barrier.
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);

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
        guard = cv1.wait(guard).unwrap();   // condvar_wait main::cv on main::m
        drop(guard);                        // mutex_unlock main::m
        pN_w1.release();                    // semaphore_release main::gN
    });

    // main::w2
    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let w2 = std::thread::spawn(move || {
        let mut guard = m2.lock().unwrap(); // mutex_lock main::m
        p12_w2.release();                   // semaphore_release main::g12
        guard = cv2.wait(guard).unwrap();   // condvar_wait main::cv on main::m
        drop(guard);                        // mutex_unlock main::m
        pN_w2.release();                    // semaphore_release main::gN
    });

    // main::notifier
    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let notifier = std::thread::spawn(move || {
        let q12_a = g12_n.acquire();        // semaphore_acquire main::g12
        let q12_b = g12_n.acquire();        // semaphore_acquire main::g12
        let guard = m3.lock().unwrap();     // mutex_lock main::m
        cv3.notify_all();                   // condvar_notify_all main::cv
        drop(guard);                        // mutex_unlock main::m
        let qN_a = gN_n.acquire();          // semaphore_acquire main::gN
        let qN_b = gN_n.acquire();          // semaphore_acquire main::gN
        // The CIR acquires consume their permits permanently, so keep them
        // instead of handing them back on drop.
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
