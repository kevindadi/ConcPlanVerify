use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    let m1 = m.clone();
    let cv1 = cv.clone();
    let g12_1 = g12.clone();
    let gN_1 = gN.clone();
    let w1 = thread::spawn(move || {
        let guard = m1.lock().unwrap();
        g12_1.release();
        let _guard = cv1.wait(guard).unwrap();
        drop(_guard);
        let _permit = gN_1.acquire();
    });

    let m2 = m.clone();
    let cv2 = cv.clone();
    let g12_2 = g12.clone();
    let gN_2 = gN.clone();
    let w2 = thread::spawn(move || {
        let guard = m2.lock().unwrap();
        g12_2.release();
        let _guard = cv2.wait(guard).unwrap();
        drop(_guard);
        let _permit = gN_2.acquire();
    });

    let m3 = m.clone();
    let cv3 = cv.clone();
    let g12_3 = g12.clone();
    let gN_3 = gN.clone();
    let notifier = thread::spawn(move || {
        let _p1 = g12_3.acquire();
        let _p2 = g12_3.acquire();
        let guard = m3.lock().unwrap();
        cv3.notify_all();
        drop(guard);
        gN_3.release();
        gN_3.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
}
