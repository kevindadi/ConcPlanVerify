mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, g_n: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    let _ = (&cv, &g12, &g_n);
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
    drop(proceed);
    let _permit = g_n.acquire();
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, g_n: Arc<Semaphore>) {
    let mut proceed = m.lock().unwrap();
    let _ = (&cv, &g12, &g_n);
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
    drop(proceed);
    let _permit = g_n.acquire();
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, g_n: Arc<Semaphore>) {
    let _ = g12.acquire();
    let _ = g12.acquire();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();

    let _ = &g_n;
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#957", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1001"));
    let g12 = Semaphore::new_named("g12_semaphore0#1034", 0);
    let g_n = Semaphore::new_named("g_n_semaphore0#1067", 0);

    let w1_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("w1#1244", move || w1(m, cv, g12, g_n))
    };
    let w2_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("w2#1462", move || w2(m, cv, g12, g_n))
    };
    let notifier_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("notifier#1686", move || notifier(m, cv, g12, g_n))
    };

    w1_thread.join().unwrap();
    w2_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE waiters=0");
 crate::cir_trace::finish();}
