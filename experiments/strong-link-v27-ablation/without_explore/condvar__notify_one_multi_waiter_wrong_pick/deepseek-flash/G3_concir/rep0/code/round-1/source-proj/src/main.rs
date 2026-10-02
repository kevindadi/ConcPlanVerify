use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};

// main::w1
fn w1(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let guard = m.lock().unwrap(); // mutex_lock m
    g12.release(); // semaphore_release g12
    let guard = cv.wait(guard).unwrap(); // condvar_wait cv/m (releases then re-acquires m)
    drop(guard); // mutex_unlock m
    gN.release(); // semaphore_release gN
}

// main::w2
fn w2(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let guard = m.lock().unwrap(); // mutex_lock m
    g12.release(); // semaphore_release g12
    let guard = cv.wait(guard).unwrap(); // condvar_wait cv/m
    drop(guard); // mutex_unlock m
    gN.release(); // semaphore_release gN
}

// main::notifier
fn notifier(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let p1 = g12.acquire(); // semaphore_acquire g12
    let p2 = g12.acquire(); // semaphore_acquire g12
    let guard = m.lock().unwrap(); // mutex_lock m
    cv.notify_all(); // condvar_notify_all cv
    drop(guard); // mutex_unlock m
    let p3 = gN.acquire(); // semaphore_acquire gN
    let p4 = gN.acquire(); // semaphore_acquire gN
    // The CIR notifier acquires and never releases these permits.
    std::mem::forget(p1);
    std::mem::forget(p2);
    std::mem::forget(p3);
    std::mem::forget(p4);
}

// main::main
fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    let h1 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        std::thread::spawn(move || w1(m, cv, g12, gN))
    };
    let h2 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        std::thread::spawn(move || w2(m, cv, g12, gN))
    };
    let h3 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        std::thread::spawn(move || notifier(m, cv, g12, gN))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE waiters=0");
}
