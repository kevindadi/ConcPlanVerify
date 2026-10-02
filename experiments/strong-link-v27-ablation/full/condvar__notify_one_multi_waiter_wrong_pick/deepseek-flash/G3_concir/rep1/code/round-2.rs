use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared condition: mutex `m` guards it, `cv` broadcasts it.
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    // g12: "both waiters are ready to wait"   (design count 0)
    // gN : "both waiters have finished"       (design count 0)
    //
    // The linked semaphore is permit based: the permits exist because
    // `Semaphore::new` created them, and only a thread that holds a permit can
    // release it.  Both waiters post g12/gN before any acquire happens, so
    // `main` takes the two permits of each semaphore before the scope starts
    // (which leaves each semaphore with the design's initial count of 0) and
    // hands one of them to the waiter that posts it.  Each waiter releases its
    // permit exactly at the design's `semaphore_release` point, so the
    // notifier still blocks until both waiters are ready, and again until both
    // waiters are finished.
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);
    let p12_w1 = g12.acquire();
    let p12_w2 = g12.acquire();
    let pN_w1 = gN.acquire();
    let pN_w2 = gN.acquire();

    // ---- w1 ----
    let m1 = m.clone();
    let cv1 = cv.clone();
    let w1 = thread::spawn(move || {
        let mut guard = m1.lock().unwrap(); // mutex_lock m
        p12_w1.release();                   // semaphore_release g12
        while !*guard {                     // condvar_wait cv (holds m)
            guard = cv1.wait(guard).unwrap();
        }
        drop(guard);                        // mutex_unlock m
        pN_w1.release();                    // semaphore_release gN
    });

    // ---- w2 ----
    let m2 = m.clone();
    let cv2 = cv.clone();
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap(); // mutex_lock m
        p12_w2.release();                   // semaphore_release g12
        while !*guard {                     // condvar_wait cv (holds m)
            guard = cv2.wait(guard).unwrap();
        }
        drop(guard);                        // mutex_unlock m
        pN_w2.release();                    // semaphore_release gN
    });

    // ---- notifier ----
    let m3 = m.clone();
    let cv3 = cv.clone();
    let g12_n = g12.clone();
    let gN_n = gN.clone();
    let notifier = thread::spawn(move || {
        let _a1 = g12_n.acquire();          // semaphore_acquire g12
        let _a2 = g12_n.acquire();          // semaphore_acquire g12
        let mut guard = m3.lock().unwrap(); // mutex_lock m
        *guard = true;                      // condition now satisfied
        cv3.notify_all();                   // condvar_notify_all cv
        drop(guard);                        // mutex_unlock m
        let _b1 = gN_n.acquire();           // semaphore_acquire gN
        let _b2 = gN_n.acquire();           // semaphore_acquire gN
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
}
