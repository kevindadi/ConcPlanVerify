mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();

    let permit = g12.acquire();
    permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    drop(proceed);
    drop(gN.acquire());
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();

    let permit = g12.acquire();
    permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    drop(proceed);
    drop(gN.acquire());
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    drop(g12.acquire());
    drop(g12.acquire());

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    let permit = gN.acquire();
    permit.release();
    let permit = gN.acquire();
    permit.release();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1091", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1135"));
    let g12 = Semaphore::new_named("g12_semaphore0#1168", 2);
    let gN = Semaphore::new_named("gN_semaphore0#1200", 2);

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        crate::cir_trace::spawn("w1#1375", move || w1(m, cv, g12, gN))
    };

    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        crate::cir_trace::spawn("w2#1591", move || w2(m, cv, g12, gN))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        crate::cir_trace::spawn("notifier#1813", move || notifier(m, cv, g12, gN))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
 crate::cir_trace::finish();}
