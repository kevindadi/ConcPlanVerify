mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>) {
    let permit = g12.acquire();
    let guard = m.lock().unwrap();
    permit.release();
    let _guard = cv.wait(guard).unwrap();
}

fn w2(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>) {
    let permit = g12.acquire();
    let guard = m.lock().unwrap();
    permit.release();
    let _guard = cv.wait(guard).unwrap();
}

fn notifier(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, g_n: Arc<Semaphore>) {
    let _g_n_permit = g_n.acquire();
    let _first = g12.acquire();
    let _second = g12.acquire();

    let _guard = m.lock().unwrap();
    cv.notify_all();
    drop(_guard);

    _g_n_permit.release();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#826", ()));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#867"));
    let g12 = Semaphore::new_named("g12_semaphore0#900", 2);
    let g_n = Semaphore::new_named("g_n_semaphore0#933", 1);

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        cir_trace::spawn("w1#1074", move || w1(m, cv, g12))
    };

    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        cir_trace::spawn("w2#1252", move || w2(m, cv, g12))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        cir_trace::spawn("notifier#1472", move || notifier(m, cv, g12, g_n))
    };

    let main_permit = g_n.acquire();
    main_permit.release();

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
