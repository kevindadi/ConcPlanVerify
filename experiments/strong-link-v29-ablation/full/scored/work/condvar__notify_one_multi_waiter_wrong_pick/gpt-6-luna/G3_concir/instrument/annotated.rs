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
    g12: Arc<Semaphore>,
    m: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    g_n_permit: concir_sync::Permit,
) {
    let permit1 = g12.acquire();
    let permit2 = g12.acquire();
    permit1.release();
    permit2.release();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    g_n_permit.release();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#927", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#971"));

    let g12: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("g12_semaphore0#1049", 2)));
    let w1_permit = g12.acquire();
    let w2_permit = g12.acquire();

    let g_n: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("g_n_semaphore0#1199", 1)));
    let g_n_permit = g_n.acquire();

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        crate::cir_trace::spawn("w1#1342", move || w1(m, cv, w1_permit))
    };
    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        crate::cir_trace::spawn("w2#1489", move || w2(m, cv, w2_permit))
    };
    let notifier_handle = {
        let g12 = Arc::clone(g12);
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        crate::cir_trace::spawn("notifier#1677", move || notifier(g12, m, cv, g_n_permit))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let _permit = g_n.acquire();
    println!("DONE waiters=0");
 crate::cir_trace::finish();}
