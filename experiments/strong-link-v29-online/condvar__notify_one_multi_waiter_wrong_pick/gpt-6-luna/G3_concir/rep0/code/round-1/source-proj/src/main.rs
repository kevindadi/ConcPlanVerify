use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, _g12: Arc<Semaphore>, _gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, _g12: Arc<Semaphore>, _gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, _g12: Arc<Semaphore>, _gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    let t1 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        thread::spawn(move || w1(m, cv, g12, gN))
    };
    let t2 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        thread::spawn(move || w2(m, cv, g12, gN))
    };
    let tn = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        thread::spawn(move || notifier(m, cv, g12, gN))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    println!("DONE waiters=0");
}
