mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, permit: concir_sync::Permit) {
    let mut proceed = m.lock().unwrap();
    permit.release();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, permit: concir_sync::Permit) {
    let mut proceed = m.lock().unwrap();
    permit.release();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn notifier(
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    g_n_permit: concir_sync::Permit,
) {
    let _permit1 = g12.acquire();
    let _permit2 = g12.acquire();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    g_n_permit.release();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#881", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#925"));

    // Keep the semaphores alive for the full lifetime of permits borrowed
    // from them and moved into the worker threads.
    let g12: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("g12_semaphore0#1130", 2)));
    let g_n: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("g_n_semaphore0#1209", 1)));

    let w1_permit = g12.acquire();
    let w2_permit = g12.acquire();
    let g_n_permit = g_n.acquire();

    let t1 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        crate::cir_trace::spawn("w1#1416", move || w1(m, cv, w1_permit))
    };

    let t2 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        crate::cir_trace::spawn("w2#1557", move || w2(m, cv, w2_permit))
    };

    let tn = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(g12);
        crate::cir_trace::spawn("notifier#1733", move || notifier(m, cv, g12, g_n_permit))
    };

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    let _g_n_permit = g_n.acquire();
    println!("DONE waiters=0");
 crate::cir_trace::finish();}
