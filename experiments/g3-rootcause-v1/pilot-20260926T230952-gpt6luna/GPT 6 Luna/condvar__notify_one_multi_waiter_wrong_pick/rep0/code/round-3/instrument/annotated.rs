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

    let permit = gN.acquire();
    permit.release();
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();

    let permit = g12.acquire();
    permit.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
    drop(proceed);

    let permit = gN.acquire();
    permit.release();
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let permit = g12.acquire();
    permit.release();
    let permit = g12.acquire();
    permit.release();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    let permit = gN.acquire();
    permit.release();
    let permit = gN.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Semaphore::new_named("g12_semaphore0", 2);
    let gN = Semaphore::new_named("gN_semaphore0", 2);

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        cir_trace::spawn("w1", move || w1(m, cv, g12, gN))
    };

    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        cir_trace::spawn("w2", move || w2(m, cv, g12, gN))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        cir_trace::spawn("notifier", move || notifier(m, cv, g12, gN))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
