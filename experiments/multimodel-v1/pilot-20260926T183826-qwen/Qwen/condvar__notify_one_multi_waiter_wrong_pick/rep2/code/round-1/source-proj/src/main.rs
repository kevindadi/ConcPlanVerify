use std::sync::{Arc, Mutex, Condvar};
use concir_sync::Semaphore;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));

    let m_w1 = Arc::clone(&m);
    let cv_w1 = Arc::clone(&cv);
    let g12_w1 = Arc::clone(&g12);

    let w1_handle = std::thread::spawn(move || {
        let mut guard = m_w1.lock().unwrap();
        let permit = g12_w1.acquire();
        permit.release();
        while !false { // condvar_wait with no predicate in CIR means wait until notified
            guard = cv_w1.wait(guard).unwrap();
            break;
        }
        drop(guard);
    });

    let m_w2 = Arc::clone(&m);
    let cv_w2 = Arc::clone(&cv);
    let g12_w2 = Arc::clone(&g12);

    let w2_handle = std::thread::spawn(move || {
        let mut guard = m_w2.lock().unwrap();
        let permit = g12_w2.acquire();
        permit.release();
        while !false {
            guard = cv_w2.wait(guard).unwrap();
            break;
        }
        drop(guard);
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);

    let notifier_handle = std::thread::spawn(move || {
        let p1 = g12_n.acquire();
        p1.release();
        let p2 = g12_n.acquire();
        p2.release();
        let _guard = m_n.lock().unwrap();
        cv_n.notify_one();
        cv_n.notify_one();
        drop(_guard);
        let pn = gN_n.acquire();
        pn.release();
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
}
