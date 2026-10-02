mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
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

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#720", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#764"));
    let g12 = Semaphore::new_named("g12_semaphore0#797", 0);
    let gN = Semaphore::new_named("gN_semaphore0#829", 0);

    let t1 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        crate::cir_trace::spawn("w1#997", move || w1(m, cv, g12, gN))
    };
    let t2 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        crate::cir_trace::spawn("w2#1205", move || w2(m, cv, g12, gN))
    };
    let tn = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        crate::cir_trace::spawn("notifier#1413", move || notifier(m, cv, g12, gN))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    println!("DONE waiters=0");
 crate::cir_trace::finish();}
