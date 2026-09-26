mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Arc::new(Semaphore::new_named("g12_semaphore0", 0));
    // gN is declared in the design but not used in any function body.
    let _gN = Arc::new(Semaphore::new_named("_gN_semaphore0", 0));

    let m_w1 = Arc::clone(&m);
    let cv_w1 = Arc::clone(&cv);
    let g12_w1 = Arc::clone(&g12);
    let w1 = cir_trace::spawn("w1", move || {
        let mut guard = m_w1.lock().unwrap();
        let permit = g12_w1.acquire();
        permit.release();
        while false {
            guard = cv_w1.wait(guard).unwrap();
        }
        drop(guard);
    });

    let m_w2 = Arc::clone(&m);
    let cv_w2 = Arc::clone(&cv);
    let g12_w2 = Arc::clone(&g12);
    let w2 = cir_trace::spawn("w2", move || {
        let mut guard = m_w2.lock().unwrap();
        let permit = g12_w2.acquire();
        permit.release();
        while false {
            guard = cv_w2.wait(guard).unwrap();
        }
        drop(guard);
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let notifier = cir_trace::spawn("notifier", move || {
        let p1 = g12_n.acquire();
        p1.release();
        let p2 = g12_n.acquire();
        p2.release();
        let guard = m_n.lock().unwrap();
        cv_n.notify_all();
        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
