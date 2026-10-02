use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let guard = m.lock().unwrap();
    g12.release();
    let _guard = cv.wait(guard).unwrap();
    drop(_guard);
    gN.release();
}

fn w2(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let guard = m.lock().unwrap();
    g12.release();
    let _guard = cv.wait(guard).unwrap();
    drop(_guard);
    gN.release();
}

fn notifier(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let p1 = g12.acquire();
    let p2 = g12.acquire();
    let guard = m.lock().unwrap();
    cv.notify_all();
    drop(guard);
    let p3 = gN.acquire();
    let p4 = gN.acquire();
    std::mem::forget(p1);
    std::mem::forget(p2);
    std::mem::forget(p3);
    std::mem::forget(p4);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_3 = Arc::clone(&g12);
    let gN_3 = Arc::clone(&gN);

    let t1 = thread::spawn(move || w1(m1, cv1, g12_1, gN_1));
    let t2 = thread::spawn(move || w2(m2, cv2, g12_2, gN_2));
    let t3 = thread::spawn(move || notifier(m3, cv3, g12_3, gN_3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE waiters=0");
}
